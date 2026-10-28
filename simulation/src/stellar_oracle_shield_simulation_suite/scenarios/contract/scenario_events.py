from stellar_oracle_shield_simulation_suite.simulator import (
    Simulator,
    DEFAULT_BASE,
    DEFAULT_QUOTE,
)

from stellar_oracle_shield_client.bindings import (
    StatusKind as OracleShieldStatusKind,
    Status as OracleShieldStatus,
    StatusChangeEvent,
    ScoreChangeEvent,
)


def run(simulator: Simulator):
    """Test events notifications."""
    operator = simulator.get_operator("main_operator")

    start = simulator.server.get_latest_ledger().sequence + 1

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 0)

    expected: ScoreChangeEvent | StatusChangeEvent
    event: ScoreChangeEvent | StatusChangeEvent | None

    expected = ScoreChangeEvent(
        base=DEFAULT_BASE,
        quote=DEFAULT_QUOTE,
        score=0,
    )

    event = simulator.wait_for_score_change_event(
        pattern=expected,
        start_ledger=start,
        timeout=15,
        timeout_ledgers=5,
    )

    assert event == expected, (event, expected)

    start = simulator.server.get_latest_ledger().sequence + 1

    operator.set_score(DEFAULT_BASE, DEFAULT_QUOTE, 100)

    expected = ScoreChangeEvent(
        base=DEFAULT_BASE,
        quote=DEFAULT_QUOTE,
        score=100,
    )

    event = simulator.wait_for_score_change_event(
        pattern=expected,
        start_ledger=start,
        timeout=15,
        timeout_ledgers=5,
    )

    assert event == expected, (event, expected)

    expected = StatusChangeEvent(
        base=DEFAULT_BASE,
        quote=DEFAULT_QUOTE,
        status=OracleShieldStatus(OracleShieldStatusKind.Healthy),
    )

    event = simulator.wait_for_status_change_event(
        pattern=expected,
        start_ledger=start,
        timeout=15,
        timeout_ledgers=5,
    )

    assert event == expected, (event, expected)
