import pytest

from .helpers import discover_scenarios

from stellar_oracle_shield_simulation_suite.simulator import DeterministicSimulator


@pytest.mark.parametrize("run_scenario", discover_scenarios("scorer"))
def test_scorer(run_scenario, deterministic_simulator: DeterministicSimulator):
    """run any scenario in contract directory (pure contract tests)"""
    run_scenario(deterministic_simulator)
