from __future__ import annotations

from stellar_sdk import Address, Keypair, MuxedAccount
from stellar_oracle_shield_client.bindings import Client as OracleShieldClient

from .operator import Operator


class Admin:
    """simulate actions done by Admin role"""

    def __init__(self, account: Keypair, client: OracleShieldClient):
        self.account = account
        self.client = client

    def add_operator(self, ope: Operator | Keypair):
        """add authorized operator"""
        operator: Address | str = ope.public_key
        muxed_account = MuxedAccount(self.account.public_key)
        self.client.add_operator(
            operator,
            muxed_account,
            self.account,
        ).sign_and_submit()

    def remove_operator(self, ope: Operator):
        """remove authorized operator"""
        operator: Address | str = ope.public_key
        muxed_account = MuxedAccount(self.account.public_key)
        self.client.remove_operator(
            operator,
            muxed_account,
            self.account,
        ).sign_and_submit()

    def set_max_staleness(self, max_staleness: int):
        """set max staleness for all pairs score
        `max_staleness` - seconds"""
        muxed_account = MuxedAccount(self.account.public_key)
        self.client.set_max_staleness(
            max_staleness,
            muxed_account,
            self.account,
        ).sign_and_submit()

    def set_max_deviation(self, max_deviation: int):
        """set maximum deviation from the median for a fresh score to be considered agreeing"""
        muxed_account = MuxedAccount(self.account.public_key)
        self.client.set_max_deviation(
            max_deviation,
            muxed_account,
            self.account,
        ).sign_and_submit()

    def set_quorum(self, quorum: int):
        """set minimum number of fresh scores agreeing with the median score. Defaults to one.
        Zero disables the quorum constraint but still requires a fresh score."""
        muxed_account = MuxedAccount(self.account.public_key)
        self.client.set_quorum(
            quorum,
            muxed_account,
            self.account,
        ).sign_and_submit()

    def hand_over_admin(self, other_admin: Admin):
        """hand over administration privileges
        current administrator will kill administration privileges
        until new_admin has accepted them"""
        new_admin: Address | str = other_admin.public_key
        muxed_account = MuxedAccount(self.account.public_key)
        self.client.hand_over_admin(
            new_admin,
            muxed_account,
            self.account,
        ).sign_and_submit()

    def accept_admin(self):
        """accept administration privileges"""
        muxed_account = MuxedAccount(self.account.public_key)
        self.client.accept_admin(
            muxed_account,
            self.account,
        ).sign_and_submit()

    @property
    def public_key(self) -> str:
        """operator public key"""
        return self.account.public_key
