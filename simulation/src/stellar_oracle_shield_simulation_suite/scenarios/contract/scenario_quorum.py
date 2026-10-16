from stellar_oracle_shield_simulation_suite.simulator import (
    Simulator,
    DEFAULT_BASE,
    DEFAULT_QUOTE,
    expect_contract_error,
)

from stellar_oracle_shield_client.bindings import (
    Error as OracleShieldError,
    StatusKind as OracleShieldStatusKind,
)


def run(simulator: Simulator):
    """Check scores are dismissed when the operator quorum is not reached."""
    operator_1 = simulator.get_operator("operator_1")
    operator_2 = simulator.get_operator("operator_2")
    operator_3 = simulator.get_operator("operator_3")
    user = simulator.get_user()
    admin = simulator.get_admin()

    # initial loose quorum setup (1/1:1)
    admin.set_quorum(1)

    operator_1.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 72)
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Healthy, status

    # tighten quorum (1/1:2)
    admin.set_quorum(2)

    with expect_contract_error() as error_returned:
        status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert (
        error_returned.error == OracleShieldError.QuorumNotReached
    ), error_returned.error

    with expect_contract_error() as error_returned:
        score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert (
        error_returned.error == OracleShieldError.QuorumNotReached
    ), error_returned.error

    # repeated report does not impact quorum (1/1:2)
    operator_1.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 73)

    with expect_contract_error() as error_returned:
        status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert (
        error_returned.error == OracleShieldError.QuorumNotReached
    ), error_returned.error

    # quorum friendly publication (2/2:2)
    operator_2.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 68)
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Healthy, status
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 70, score

    # quorum unfriendly publication(1/2:2)
    operator_2.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 2)
    with expect_contract_error() as error_returned:
        status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert (
        error_returned.error == OracleShieldError.QuorumNotReached
    ), error_returned.error
    with expect_contract_error() as error_returned:
        score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert (
        error_returned.error == OracleShieldError.QuorumNotReached
    ), error_returned.error

    # quorum friendly publication (2/3:2)
    operator_3.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 70)
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Healthy, status
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 70, score

    # stale scores are excluded from quorum (0/3:2)
    admin.set_max_staleness(1)
    simulator.wait_for_ledgers(1)  # assuming ledger schedule > 1s

    with expect_contract_error() as error_returned:
        score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert error_returned.error == OracleShieldError.StaleInput, error_returned.error

    # republish one (1/3:2)
    operator_1.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 73)

    with expect_contract_error() as error_returned:
        score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert (
        error_returned.error == OracleShieldError.QuorumNotReached
    ), error_returned.error

    # republish one and two (2/3:2)
    operator_1.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 73)
    operator_3.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 70)

    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 71, score
