from .config import Config
from .simulator import Simulator
from .helpers import derive_contract_id

from stellar_sdk import Keypair

DEPLOYER_ACCOUNT_MNEMONIC = (
    "three lemon call version fortune shrug tilt afraid fame congress repeat filter"
)


class DeterministicSimulator(Simulator):
    """A simulator with deterministic contract_id"""

    def __init__(self, config: Config):
        super().__init__(config)

        # fixed deployer and fixed salt will lead to deterministic contract address
        self.admin_account = super().create_funded_account(
            keypair=Keypair.from_mnemonic_phrase(DEPLOYER_ACCOUNT_MNEMONIC)
        )
        self.deployment_salt = bytes.fromhex("01" * 32)

        target_contract_id = derive_contract_id(
            self.admin_account.public_key,
            self.deployment_salt,
            self.config.node.network_passphrase,
        )
        print(f"deterministic contract_id: {target_contract_id}")
