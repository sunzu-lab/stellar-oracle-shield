from stellar_oracle_shield_simulation_suite.simulator import (
    Simulator,
    OracleShieldError,
    expect_needs_more_signatures,
)


def run(simulator: Simulator):
    """Test admin priviledge handover procedure"""
    admin = simulator.get_admin()
    new_admin = simulator.new_admin()

    admin.set_quorum(1)
    with expect_needs_more_signatures() as error_returned:
        new_admin.set_quorum(2)
    assert error_returned.error

    admin.hand_over_admin(new_admin)
    # still no change
    admin.set_quorum(1)
    with expect_needs_more_signatures() as error_returned:
        new_admin.set_quorum(2)
    assert error_returned.error

    new_admin.accept_admin()
    # switch
    new_admin.set_quorum(1)
    with expect_needs_more_signatures() as error_returned:
        admin.set_quorum(2)
    assert error_returned.error
