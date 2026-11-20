from stellar_sdk import Keypair
from stellar_oracle_shield_client.bindings import Client as OracleShieldClient


class ClientAccount:

    def __init__(self, account: Keypair, client: OracleShieldClient):
        self.account = account
        self.client = client

    @property
    def public_key(self) -> str:
        """operator public key"""
        return self.account.public_key
