from stellar_sdk import Address, Keypair, MuxedAccount, xdr
from stellar_oracle_shield_client.bindings import (
    Client as OracleShieldClient,
    StatusKind as OracleShieldStatusKind,
)

from .client import ClientAccount


class User(ClientAccount):

    def version(self) -> str:
        """retrieve the version of the contract"""
        version = self.client.version(
            self.public_key,
        ).result()

        if isinstance(version, xdr.SCVal):
            raise TypeError("Raw value returned")

        return ".".join([str(v) for v in version])

    def get_score(self, base: Address | str, quote: Address | str) -> int:
        """get score of a pair
        `base` - SAC address of an asset
        `quote` - SAC address of an asset

        return the score
        raises if
        - pair is not covered
        - no fresh operator reports remain
        - median status consensus is below quorum"""
        score = self.client.get_score(
            base,
            quote,
            self.public_key,
        ).result()

        if isinstance(score, xdr.SCVal):
            raise TypeError("Raw value returned")

        return score

    def get_status(
        self, base: Address | str, quote: Address | str
    ) -> OracleShieldStatusKind:
        """get health status of a pair
        `base` - SAC address of an asset
        `quote` - SAC address of an asset

        return the score
        raises if
        - pair is not covered
        - no fresh operator reports remain
        - median status consensus is below quorum"""
        status = self.client.get_status(
            base,
            quote,
            self.public_key,
        ).result()

        if isinstance(status, xdr.SCVal):
            raise TypeError("Raw value returned")

        return status.kind
