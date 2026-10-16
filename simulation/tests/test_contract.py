from importlib import import_module
from pkgutil import iter_modules

import pytest

from stellar_oracle_shield_simulation_suite.simulator import Simulator
from stellar_oracle_shield_simulation_suite.scenarios import contract


def discover_scenarios():
    cases = []

    for info in sorted(iter_modules(contract.__path__), key=lambda m: m.name):
        if not info.name.startswith("scenario_"):
            continue

        module = import_module(f"{contract.__name__}.{info.name}")
        run = getattr(module, "run", None)

        if not callable(run):
            raise pytest.UsageError(
                f"{module.__name__} must export a callable run(context)"
            )

        cases.append(pytest.param(run, id=info.name.removeprefix("scenario_")))

    if not cases:
        raise pytest.UsageError("No scenarios discovered")

    return cases


@pytest.mark.parametrize("run_scenario", discover_scenarios())
def test_scenario(run_scenario, simulator: Simulator):
    """run any scenario in contract directory (pure contract tests)"""
    run_scenario(simulator)
