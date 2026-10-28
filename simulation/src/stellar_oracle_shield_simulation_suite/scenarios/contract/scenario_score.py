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
    """Check scores boundaries and status mapping."""
    operator = simulator.get_operator("main_operator")
    user = simulator.get_user()

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 0)
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 0, score
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Unsafe, status

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 32)
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 32, score
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Unsafe, status

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 33)
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 33, score
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Degraded, status

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 65)
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 65, score
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Degraded, status

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 66)
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 66, score
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Healthy, status

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 100)
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 100, score
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Healthy, status

    with expect_contract_error() as error_returned:
        operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 101)
    assert error_returned.error == OracleShieldError.ScoreBounds, error_returned.error
