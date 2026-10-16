from stellar_sdk import Address, Keypair, MuxedAccount, SorobanServer, scval, xdr
from stellar_oracle_shield_client.bindings import Client as OracleShieldClient


class Operator:

    def __init__(self, name: str, account: Keypair, client: OracleShieldClient):
        self.name = name
        self.account = account
        self.client = client

    def set_score(self, base: Address, quote: Address, score: int):
        """set score of a pair in legacy single operator mode
        `base` - SAC address of an asset
        `quote` - SAC address of an asset
        `score` - [0-100] scoring. 0 the more unsafe, 100 the healthier"""
        self.set_scores([(base, quote, score)])

    def set_scores(self, scores: list[tuple[Address | str, Address | str, int]]):
        """scores for multiple pairs
        `operator` - operator generating scores
        `scores` - vector of scores with the struct (base, quote, score)
        base - SAC address of an asset
        quote - SAC address of an asset
        score - [0-100] scoring. 0 the more unsafe, 100 the healthier"""
        muxed_account = MuxedAccount(self.account.public_key)
        self.client.set_scores(
            self.account.public_key,
            scores,
            muxed_account,
            self.account,
        ).sign_and_submit()

    @property
    def public_key(self) -> str:
        """operator public key"""
        return self.account.public_key
