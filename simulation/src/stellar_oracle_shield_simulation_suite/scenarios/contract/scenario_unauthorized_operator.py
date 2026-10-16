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
    """Check unauthorized operator submissions are rejected."""
    operator = simulator.get_operator("main_operator", register=False)
    user = simulator.get_user()
    admin = simulator.get_admin()

    with expect_contract_error() as error_returned:
        operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 0)
    assert (
        error_returned.error == OracleShieldError.UnauthorizedOperator
    ), error_returned.error

    admin.add_operator(operator)

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 0)
    score = user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)
    assert score == 0, score
