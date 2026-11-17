from stellar_sdk import Address, Keypair, MuxedAccount
from stellar_oracle_shield_client.bindings import Client as OracleShieldClient

from .client import ClientAccount


class Operator(ClientAccount):

    def __init__(self, name: str, account: Keypair, client: OracleShieldClient):
        super().__init__(account, client)
        self.name = name

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
        self.client.set_scores(
            self.account.public_key,
            scores,
            self.public_key,
            self.account,
        ).sign_and_submit()
