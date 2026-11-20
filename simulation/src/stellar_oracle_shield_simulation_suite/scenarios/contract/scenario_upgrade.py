from stellar_oracle_shield_simulation_suite.simulator import (
    Simulator,
    expect_needs_more_signatures,
    expect_contract_error,
)

from stellar_oracle_shield_client.bindings import (
    Error as OracleShieldError,
)

import time
import re

SENTINEL_WASM = bytes.fromhex(
    "0061736d01000000010b0260027e7e017e6000017e0207010176016700000302010105030100100611027f01418080c0000b7f00418080c0000b071803066d656d6f727902000776657273696f6e0001015f03010a50014e02017f017e230041206b2200240020004284808080f00037031820004284808080f00037031020004284808080f000370308200041086aad4220864204844284808080301000200041206a24000b001e11636f6e7472616374656e766d6574617630000000000000001c000000000093010e636f6e74726163746d65746176300000000000000005727376657200000000000006312e39372e3100000000000000000008727373646b7665720000002f32382e302e302334386435303637313266393634303934643134313736653266306230326166636431303534353637000000000000000012727373646b5f737065635f7368616b696e670000000000013200000000530e636f6e74726163746d65746176300000000000000006636c6976657200000000002f33302e302e31236632613762373431373563333435383566336233306131616266343261326434316362636333393000003f0e636f6e747261637473706563763000000000000000000000000776657273696f6e000000000000000001000003ed00000003000000040000000400000004"
)


def is_version(s: str) -> bool:
    return bool(re.fullmatch(r"\d+\.\d+\.\d+", s, flags=re.ASCII))


def run(simulator: Simulator):
    """Test upgradability"""
    admin = simulator.get_admin()
    user_as_admin = simulator.get_unauthorized_admin()
    timelock_delay_seconds = admin.activate_timelock()

    user = simulator.get_user()
    version = user.version()
    assert is_version(version), version

    wasm_hash = simulator.upload_contract(SENTINEL_WASM)

    # user is not authorized to schedule an upgrade
    with expect_needs_more_signatures() as error_returned:
        user_as_admin.schedule_upgrade(wasm_hash)
    assert error_returned.error

    admin.schedule_upgrade(wasm_hash)

    # upgrade can't be execute before time lock delay
    with expect_contract_error() as contract_error_returned:
        admin.execute_upgrade()
    assert (
        contract_error_returned.error == OracleShieldError.TimeLocked
    ), contract_error_returned.error

    time.sleep(timelock_delay_seconds)

    # user is authorized to execute scheduled upgrade
    user_as_admin.execute_upgrade()

    user = simulator.get_user()
    version = user.version()
    assert version == "7.7.7", version
