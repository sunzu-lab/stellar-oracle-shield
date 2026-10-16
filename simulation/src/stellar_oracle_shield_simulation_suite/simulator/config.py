from pydantic import BaseModel, ConfigDict

from pathlib import Path

DEFAULT_STELLAR_RPC_URL = "http://localhost:8000/soroban/rpc"
DEFAULT_FRIENDBOT_URL = "http://localhost:8000/friendbot"
DEFAULT_NETWORK_PASSPHRASE = "Standalone Network ; February 2017"


class NodeConfig(BaseModel):
    model_config = ConfigDict(
        from_attributes=False,
    )

    rpc_url: str = DEFAULT_STELLAR_RPC_URL
    friendbot_url: str = DEFAULT_FRIENDBOT_URL
    network_passphrase: str = DEFAULT_NETWORK_PASSPHRASE


class ContractConfig(BaseModel):
    model_config = ConfigDict(
        from_attributes=False,
    )

    """wasm of the contract to test"""
    wasm_path: Path


DEFAULT_SCORER_HOST = "localhost"
DEFAULT_SCORER_PORT = 8696


class ScorerConfig(BaseModel):
    model_config = ConfigDict(
        from_attributes=False,
    )

    host: str = DEFAULT_SCORER_HOST
    port: int = DEFAULT_SCORER_PORT


class Config(BaseModel):
    model_config = ConfigDict(
        from_attributes=False,
    )

    """stellar node configuration (usually local node)"""
    node: NodeConfig = NodeConfig()
    """contract configuration"""
    contract: ContractConfig
    """local scorer configuration"""
    scorer: ScorerConfig = ScorerConfig()
