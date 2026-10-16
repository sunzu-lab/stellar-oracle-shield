from stellar_sdk import Address, Keypair, MuxedAccount, SorobanServer, scval, xdr
from stellar_oracle_shield_client.bindings import (
    Client as OracleShieldClient,
    StatusKind as OracleShieldStatusKind,
)


class User:

    def __init__(self, account: Keypair, client: OracleShieldClient):
        self.account = account
        self.client = client

    def version(self) -> str:
        """retrieve the version of the contract"""
        muxed_account = MuxedAccount(self.account.public_key)
        version = self.client.version(
            muxed_account,
        ).result()

        if isinstance(version, xdr.SCVal):
            raise TypeError(f"Raw value returned")

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
        muxed_account = MuxedAccount(self.account.public_key)
        score = self.client.get_score(
            base,
            quote,
            muxed_account,
        ).result()

        if isinstance(score, xdr.SCVal):
            raise TypeError(f"Raw value returned")

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
        muxed_account = MuxedAccount(self.account.public_key)
        status = self.client.get_status(
            base,
            quote,
            muxed_account,
        ).result()

        if isinstance(status, xdr.SCVal):
            raise TypeError(f"Raw value returned")

        return status.kind
