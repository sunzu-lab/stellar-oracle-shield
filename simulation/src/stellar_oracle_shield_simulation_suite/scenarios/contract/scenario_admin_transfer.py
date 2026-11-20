from stellar_oracle_shield_simulation_suite.simulator import (
    Simulator,
    expect_needs_more_signatures,
    expect_contract_error,
)

from stellar_oracle_shield_client.bindings import (
    Error as OracleShieldError,
)

import time


def run(simulator: Simulator):
    """Test admin priviledge handover procedure"""
    admin = simulator.get_admin()
    new_admin = simulator.new_admin()

    timelock_delay_seconds = admin.activate_timelock()

    admin.schedule_set_max_staleness(1)
    admin.cancel_set_parameters()
    with expect_needs_more_signatures() as error_returned:
        new_admin.schedule_set_max_staleness(2)
    assert error_returned.error

    admin.transfer_admin(new_admin)
    # still no change
    admin.schedule_set_max_staleness(1)
    admin.cancel_set_parameters()
    with expect_needs_more_signatures() as error_returned:
        new_admin.schedule_set_max_staleness(2)
    assert error_returned.error

    with expect_contract_error() as contract_error_returned:
        new_admin.accept_admin()
    assert (
        contract_error_returned.error == OracleShieldError.TimeLocked
    ), error_returned.error

    time.sleep(timelock_delay_seconds)

    with expect_contract_error() as contract_error_returned:
        new_admin.accept_admin()
    assert contract_error_returned.error == None, error_returned.error

    # switched
    new_admin.schedule_set_max_staleness(1)
    new_admin.cancel_set_parameters()
    with expect_needs_more_signatures() as error_returned:
        admin.schedule_set_max_staleness(2)
    assert error_returned.error
