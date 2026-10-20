import pytest

from importlib import import_module
from pkgutil import iter_modules

from stellar_oracle_shield_simulation_suite import scenarios


def discover_scenarios(familly: str):
    cases = []

    familly_package = import_module(f"{scenarios.__name__}.{familly}")
    for info in sorted(iter_modules(familly_package.__path__), key=lambda m: m.name):
        if not info.name.startswith("scenario_"):
            continue

        module = import_module(f"{scenarios.__name__}.{familly}.{info.name}")
        run = getattr(module, "run", None)

        if not callable(run):
            raise pytest.UsageError(
                f"{module.__name__} must export a callable run(context)"
            )

        cases.append(pytest.param(run, id=info.name.removeprefix("scenario_")))

    if not cases:
        raise pytest.UsageError("No scenarios discovered")

    return cases
