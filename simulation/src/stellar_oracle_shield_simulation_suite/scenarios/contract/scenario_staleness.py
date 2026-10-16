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
    """Check scores are dismissed when too old."""
    operator = simulator.get_operator("main_operator")
    user = simulator.get_user()
    admin = simulator.get_admin()

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 12)
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 12, score
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Unsafe, status

    # tighten staleness acceptability
    admin.set_max_staleness(1)
    simulator.wait_for_ledgers(1)  # assuming ledger schedule > 1s

    # stale score
    with expect_contract_error() as error_returned:
        score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert error_returned.error == OracleShieldError.StaleInput, error_returned.error
    # consequently, stale status
    with expect_contract_error() as error_returned:
        status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert error_returned.error == OracleShieldError.StaleInput, error_returned.error

    # loosen staleness acceptability
    admin.set_max_staleness(86400)

    # back to acceptable
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 12, score
    status = user.get_status(DEFAULT_BASE, DEFAULT_QUOTE)
    assert status == OracleShieldStatusKind.Unsafe, status
