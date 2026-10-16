from .admin import Admin
from .config import Config
from .helpers import build_constructor_args
from .operator import Operator
from .user import User

from pathlib import Path
import secrets
import time

import requests
from stellar_sdk import Address, Keypair, SorobanServer, scval
from stellar_sdk.contract import ContractClient
from stellar_sdk.exceptions import AccountNotFoundException
from stellar_sdk.sep.contract_spec import ContractSpec

from stellar_sdk.soroban_rpc import EventInfo, EventFilter, EventFilterType

from stellar_oracle_shield_client.bindings import (
    Client as OracleShieldClient,
    StatusChangeEvent,
    ScoreChangeEvent,
)

from collections.abc import Callable, Sequence
from typing import TypeVar

EventT = TypeVar("EventT")


DEFAULT_BASE: Address = Address(
    "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA"
)
DEFAULT_QUOTE: Address = Address(
    "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75"
)

class Simulator:

    def __init__(self, config: Config):
        self.config = config

        self.server = SorobanServer(self.config.node.rpc_url)
        self.http = requests.Session()

        self.admin_account: Keypair | None = None
        self.admin: Admin | None = None

        self.additional_accounts: dict[str, Keypair] = {}
        self.operator: dict[str, Operator] = {}
        self.user: User | None = None

        self.contract_id: str | None = None
        self.contract_client: OracleShieldClient | None = None

    def close(self) -> None:
        try:
            self.server.close()
            if self.contract_client:
                self.contract_client.server.close()
        finally:
            self.http.close()

    def create_funded_account(
        self,
        count: int = 3,
        visibility_timeout: float = 30,
    ) -> Keypair:
        """Create fresh accounts and wait until RPC can read each one."""
        if visibility_timeout <= 0:
            raise ValueError("visibility_timeout must be positive")

        account = Keypair.random()

        response = self.http.get(
            self.config.node.friendbot_url,
            params={"addr": account.public_key},
            timeout=30,
        )
        response.raise_for_status()

        # Friendbot and RPC may observe the ledger at different times.
        deadline = time.monotonic() + visibility_timeout

        while True:
            try:
                self.server.load_account(account.public_key)
                break
            except AccountNotFoundException as exc:
                if time.monotonic() >= deadline:
                    raise TimeoutError(
                        f"Funded account not visible in RPC: " f"{account.public_key}"
                    ) from exc

                time.sleep(0.5)

        return account

    def get_admin_account(self) -> Keypair:
        """Return admin account instance."""
        if self.admin_account is None:
            self.admin_account = self.create_funded_account()

        return self.admin_account

    def get_account(self, name: str) -> Keypair:
        """Return named account instance. On the fly creation"""
        if name not in self.additional_accounts:
            self.additional_accounts[name] = self.create_funded_account()

        return self.additional_accounts[name]

    def deploy_contract(
        self,
        submit_timeout: int = 120,
    ) -> str:
        """Upload compiled WASM, then deploy a fresh contract instance.
        Assumes constructor support call with a unique arg: admin (all others defaulting)
        Returns contract id"""

        deployer = self.get_admin_account()
        wasm = Path(self.config.contract.wasm_path).read_bytes()

        wasm_hash = ContractClient.upload_contract_wasm(
            contract=wasm,
            source=deployer.public_key,
            signer=deployer,
            soroban_server=self.server,
            network_passphrase=self.config.node.network_passphrase,
            submit_timeout=submit_timeout,
        )

        spec = ContractSpec.from_wasm(wasm)
        constructor_args = build_constructor_args(
            spec, {"admin": scval.to_address(deployer.public_key)}
        )

        contract_id = ContractClient.create_contract(
            wasm_id=wasm_hash,
            source=deployer.public_key,
            signer=deployer,
            soroban_server=self.server,
            network_passphrase=self.config.node.network_passphrase,
            constructor_args=constructor_args,
            salt=secrets.token_bytes(32),
            submit_timeout=submit_timeout,
        )

        return contract_id

    def wait_for_ledgers(
        self,
        count: int = 1,
        *,
        after: int | None = None,
        timeout: float = 30,
        poll_interval: float = 0.2,
    ) -> int:
        """Wait for RPC to observe `count` ledgers after a baseline.

        If `after` is omitted, use the latest observed ledger at entry.
        Return the observed sequence once the target is reached.
        RPC errors propagate.
        """
        if count < 1:
            raise ValueError("count must be at least 1")
        if timeout <= 0:
            raise ValueError("timeout must be positive")
        if poll_interval <= 0:
            raise ValueError("poll_interval must be positive")

        deadline = time.monotonic() + timeout
        observed = self.server.get_latest_ledger().sequence
        baseline = observed if after is None else after
        target = baseline + count

        while observed < target:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(
                    f"Waiting for ledger {target}: "
                    f"RPC last observed {observed}, baseline {baseline}"
                )

            time.sleep(min(poll_interval, remaining))
            observed = self.server.get_latest_ledger().sequence

        return observed

    def _wait_for_event(
        self,
        *,
        topics: Sequence[str],
        parse: Callable[[EventInfo], EventT],
        start_ledger: int | None = None,
        timeout: float = 30,
        timeout_ledgers: int | None = None,
        poll_interval: float = 0.2,
    ) -> EventT | None:
        """Return the first matching event, or None when a limit is reached.

        A supplied pattern matches base, quote, and status exactly.
        RPC and event-decoding errors propagate.
        """
        if timeout <= 0:
            raise ValueError("timeout must be positive")
        if poll_interval <= 0:
            raise ValueError("poll_interval must be positive")
        if start_ledger is not None and start_ledger < 1:
            raise ValueError("start_ledger must be positive")
        if timeout_ledgers is not None and timeout_ledgers < 1:
            raise ValueError("timeout_ledgers must be positive")

        deadline = time.monotonic() + timeout
        contract_id = self.get_contract_id()
        latest = self.server.get_latest_ledger().sequence
        start = latest if start_ledger is None else start_ledger

        # Exclusive upper bound, if a ledger limit was requested.
        end = None if timeout_ledgers is None else start + timeout_ledgers

        event_filter = EventFilter(
            event_type=EventFilterType.CONTRACT,
            contract_ids=[contract_id],
            topics=[topics],
        )

        cursor: str | None = None
        seen: set[str] = set()
        page_size = 100

        while time.monotonic() < deadline:
            # RPC rejects a start ledger beyond its latest observed ledger.
            if cursor is None and latest < start:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    return None

                time.sleep(min(poll_interval, remaining))
                latest = self.server.get_latest_ledger().sequence
                continue

            response = self.server.get_events(
                start_ledger=start if cursor is None else None,
                cursor=cursor,
                filters=[event_filter],
                limit=page_size,
            )
            latest = response.latest_ledger

            for raw_event in response.events:
                # Events are returned in ledger/event order.
                if end is not None and raw_event.ledger >= end:
                    return None

                if raw_event.id in seen:
                    continue
                seen.add(raw_event.id)

                event = parse(raw_event)
                return event

            cursor = response.cursor

            # Drain full pages immediately, including when the final ledger
            # has already been observed. A match may be on the next page.
            if len(response.events) == page_size:
                continue

            if end is not None and latest >= end - 1:
                return None

            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return None

            time.sleep(min(poll_interval, remaining))

        return None

    def wait_for_status_change_event(
        self,
        pattern: StatusChangeEvent | None = None,
        start_ledger: int | None = None,
        *,
        timeout: float = 30,
        timeout_ledgers: int | None = None,
    ) -> StatusChangeEvent | None:
        topics = (
            StatusChangeEvent.topic_filter()
            if pattern is None
            else StatusChangeEvent.topic_filter(
                base=pattern.base,
                quote=pattern.quote,
            )
        )

        return self._wait_for_event(
            topics=topics,
            parse=StatusChangeEvent.parse,
            start_ledger=start_ledger,
            timeout=timeout,
            timeout_ledgers=timeout_ledgers,
        )

    def wait_for_score_change_event(
        self,
        pattern: ScoreChangeEvent | None = None,
        start_ledger: int | None = None,
        *,
        timeout: float = 30,
        timeout_ledgers: int | None = None,
    ) -> ScoreChangeEvent | None:
        topics = (
            ScoreChangeEvent.topic_filter()
            if pattern is None
            else ScoreChangeEvent.topic_filter(
                base=pattern.base,
                quote=pattern.quote,
            )
        )

        return self._wait_for_event(
            topics=topics,
            parse=ScoreChangeEvent.parse,
            start_ledger=start_ledger,
            timeout=timeout,
            timeout_ledgers=timeout_ledgers,
        )

    def get_contract_id(self) -> str:
        """Return contract instance id"""
        if self.contract_id is None:
            self.contract_id = self.deploy_contract()

        return self.contract_id

    def get_contract_client(self) -> OracleShieldClient:
        """Return client over contract instance"""
        if self.contract_client is None:
            contract_id = self.get_contract_id()
            self.contract_client = OracleShieldClient(
                contract_id,
                self.config.node.rpc_url,
                self.config.node.network_passphrase,
            )

        return self.contract_client

    def get_admin(self) -> Admin:
        """Return admin instance"""
        if self.admin is None:
            account = self.get_admin_account()
            client = self.get_contract_client()

            self.admin = Admin(account, client)

        return self.admin

    def set_admin(self, admin: Admin):
        """Set new admin instance"""
        self.admin = admin
        self.admin_account = admin.account

    def new_admin(self) -> Admin:
        """Return new admin candidate"""
        account = self.create_funded_account()
        client = self.get_contract_client()

        return Admin(account, client)

    def get_operator(self, name: str, register: bool = True) -> Operator:
        """Return named operator. On the fly creation.
        `register` - if True and new operator, automatically authorize it"""
        if self.operator.get(name) is None:
            account = self.get_account(name)
            client = self.get_contract_client()

            operator = Operator(name, account, client)
            self.operator[name] = operator

            if register:
                admin = self.get_admin()
                admin.add_operator(operator)

        return self.operator[name]

    def get_user(self) -> User:
        """Return user instance"""
        if self.user is None:
            account = self.get_account("_user")
            client = self.get_contract_client()

            self.user = User(account, client)

        return self.user
