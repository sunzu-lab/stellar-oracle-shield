# scenario close to 22 February 2026 YieldBlox pool exploit on Blend V2
# we use `mid price` here rather than `vwap 5 minutes` like reflector
# scorer can be easilly reconfigured to rely on `last price` or `vwap 1 minute`

from stellar_sdk import Address

from stellar_oracle_shield_simulation_suite.simulator import (
    Simulator,
    expect_contract_error,
)

from stellar_oracle_shield_client.bindings import (
    StatusKind as OracleShieldStatusKind,
    Status as OracleShieldStatus,
    StatusChangeEvent,
    ScoreChangeEvent,
)

USTRY_SAC: Address = Address("CBLV4ATSIWU67CFSQU2NVRKINQIKUZ2ODSZBUJTJ43VJVRSBTZYOPNUR")
USDC_SAC: Address = Address("CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75")


def run(simulator: Simulator):
    """Check mid_price alter score"""
    expected: ScoreChangeEvent | StatusChangeEvent
    event: ScoreChangeEvent | StatusChangeEvent | None

    scorer_account = simulator.get_scorer_account()
    user = simulator.get_user()
    admin = simulator.get_admin()
    admin.add_operator(scorer_account)

    # initial state
    indicator_engine = simulator.get_indicator_engine()
    indicator_engine.set_midprice("etherfuse", "USTRY-USDC", 1.06)
    indicator_engine.set_midprice("stellar", "USTRY.GCRYUGD5N-USDC.GA5ZSEJYB", 1.056)

    simulator.wait_for_ledgers(4)

    with expect_contract_error() as error_returned:
        score = user.get_score(USTRY_SAC, USDC_SAC)
        assert score == 62, score
    if error_returned.error:
        raise ValueError(
            f"contract error {error_returned.error!r} - is scorer running ?"
        )

    # price manipulation
    start = simulator.server.get_latest_ledger().sequence + 1

    indicator_engine.set_midprice("stellar", "USTRY.GCRYUGD5N-USDC.GA5ZSEJYB", 107)

    expected = ScoreChangeEvent(
        base=USTRY_SAC,
        quote=USDC_SAC,
        score=0,
    )
    event = simulator.wait_for_score_change_event(expected, start)
    assert event == expected, (event, expected)

    expected = StatusChangeEvent(
        base=USTRY_SAC,
        quote=USDC_SAC,
        status=OracleShieldStatus(OracleShieldStatusKind.Unsafe),
    )
    event = simulator.wait_for_status_change_event(expected, start)
    assert event == expected, (event, expected)

    # price restoration
    start = simulator.server.get_latest_ledger().sequence + 1

    indicator_engine.set_midprice("stellar", "USTRY.GCRYUGD5N-USDC.GA5ZSEJYB", 1.057)

    expected = ScoreChangeEvent(
        base=USTRY_SAC,
        quote=USDC_SAC,
        score=71,
    )
    event = simulator.wait_for_score_change_event(expected, start)
    assert event == expected, (event, expected)

    expected = StatusChangeEvent(
        base=USTRY_SAC,
        quote=USDC_SAC,
        status=OracleShieldStatus(OracleShieldStatusKind.Healthy),
    )
    event = simulator.wait_for_status_change_event(expected, start)
    assert event == expected, (event, expected)
