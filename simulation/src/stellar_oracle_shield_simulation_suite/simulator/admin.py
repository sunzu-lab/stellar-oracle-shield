from __future__ import annotations

from stellar_sdk import Address, Keypair, MuxedAccount, xdr
from stellar_oracle_shield_client.bindings import (
    Client as OracleShieldClient,
    GovernanceParameters,
    Parameters,
)

from .client import ClientAccount
from .operator import Operator

import time


class Admin(ClientAccount):
    """simulate actions done by Admin role"""

    def schedule_set_parameters(self, parameters: Parameters):
        self.client.schedule_set_parameters(
            parameters,
            self.public_key,
            self.account,
        ).sign_and_submit()

    def execute_set_parameters(self):
        self.client.execute_set_parameters(
            self.public_key,
            self.account,
        ).sign_and_submit()

    def cancel_set_parameters(self):
        self.client.cancel_set_parameters(
            self.public_key,
            self.account,
        ).sign_and_submit()

    def add_operator(self, ope: Operator | Keypair):
        """add authorized operator"""
        operator: Address | str = ope.public_key
        self.client.schedule_add_operator(
            operator,
            self.public_key,
            self.account,
        ).sign_and_submit()
        self.execute_set_parameters()

    def remove_operator(self, ope: Operator):
        """remove authorized operator"""
        operator: Address | str = ope.public_key
        self.client.schedule_remove_operator(
            operator,
            self.public_key,
            self.account,
        ).sign_and_submit()
        self.execute_set_parameters()

    def schedule_set_max_staleness(self, max_staleness: int):
        """schedule set max staleness for all pairs score
        `max_staleness` - seconds"""
        self.client.schedule_set_max_staleness(
            max_staleness,
            self.public_key,
            self.account,
        ).sign_and_submit()

    def set_max_staleness(self, max_staleness: int):
        """set max staleness for all pairs score
        `max_staleness` - seconds"""
        self.schedule_set_max_staleness(max_staleness)
        self.execute_set_parameters()

    def schedule_set_max_deviation(self, max_deviation: int):
        """schedule set maximum deviation from the median for a fresh score to be considered agreeing"""
        self.client.schedule_set_max_deviation(
            max_deviation,
            self.public_key,
            self.account,
        ).sign_and_submit()

    def set_max_deviation(self, max_deviation: int):
        """set maximum deviation from the median for a fresh score to be considered agreeing"""
        self.schedule_set_max_deviation(max_deviation)
        self.execute_set_parameters()

    def schedule_set_quorum(self, quorum: int):
        """shedule minimum number of fresh scores agreeing with the median score. Defaults to one.
        Zero disables the quorum constraint but still requires a fresh score."""
        self.client.schedule_set_quorum(
            quorum,
            self.public_key,
            self.account,
        ).sign_and_submit()

    def set_quorum(self, quorum: int):
        """set minimum number of fresh scores agreeing with the median score. Defaults to one.
        Zero disables the quorum constraint but still requires a fresh score."""
        self.schedule_set_quorum(quorum)
        self.execute_set_parameters()

    def transfer_admin(self, other_admin: Admin):
        """hand over administration privileges
        current administrator will kill administration privileges
        until new_admin has accepted them"""
        new_admin: Address | str = other_admin.public_key
        self.client.schedule_admin_transfer(
            new_admin,
            self.public_key,
            self.account,
        ).sign_and_submit()

    def accept_admin(self):
        """accept administration privileges"""
        self.client.execute_admin_transfer(
            self.public_key,
            self.account,
        ).sign_and_submit()

    def schedule_set_governance_params(self, governance_params: GovernanceParameters):
        self.client.schedule_set_governance_params(
            governance_params, self.public_key, self.account
        ).sign_and_submit()

    def execute_set_governance_params(self):
        self.client.execute_set_governance_params(
            self.public_key,
            self.account,
        ).sign_and_submit()

    def schedule_upgrade(
        self,
        new_wasm_hash: bytes,
    ):
        self.client.schedule_upgrade(
            new_wasm_hash, self.public_key, self.account
        ).sign_and_submit()

    def execute_upgrade(
        self,
    ):
        self.client.execute_upgrade(self.public_key, self.account).sign_and_submit()

    def activate_timelock(self) -> float:
        governance_params = self.client.get_governance_params(self.public_key).result()

        if isinstance(governance_params, xdr.SCVal):
            raise TypeError("Raw value returned")

        governance_params.timelock_delay_seconds = 5

        self.schedule_set_governance_params(governance_params)
        self.execute_set_governance_params()

        return governance_params.timelock_delay_seconds

    def deactivate_timelock(self):
        governance_params = self.client.get_governance_params(self.public_key).result()

        if isinstance(governance_params, xdr.SCVal):
            raise TypeError("Raw value returned")

        timelock_delay_seconds = governance_params.timelock_delay_seconds
        governance_params.timelock_delay_seconds = 0

        self.schedule_set_governance_params(governance_params)
        time.sleep(timelock_delay_seconds)
        self.execute_set_governance_params()

    @property
    def public_key(self) -> str:
        """operator public key"""
        return self.account.public_key
