import pytest

from .helpers import discover_scenarios

from stellar_oracle_shield_simulation_suite.simulator import Simulator


@pytest.mark.parametrize("run_scenario", discover_scenarios("contract"))
def test_contract(run_scenario, simulator: Simulator):
    """run any scenario in contract directory (pure contract tests)"""
    run_scenario(simulator)
