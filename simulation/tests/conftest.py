import pytest
import yaml
import inspect

from pathlib import Path
from functools import wraps

from collections.abc import Generator

from pydantic import BaseModel, ConfigDict

from stellar_oracle_shield_simulation_suite.simulator import (
    DeterministicSimulator,
    Simulator,
    Config as SimulatorConfig,
)


class Config(BaseModel):
    model_config = ConfigDict(
        from_attributes=False,
    )

    simulator: SimulatorConfig


def _with_docstring(test_function, docstring):
    @wraps(test_function)
    def wrapped(*args, **kwargs):
        return test_function(*args, **kwargs)

    wrapped.__doc__ = docstring
    return wrapped


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    for item in items:
        if not isinstance(item, pytest.Function):
            continue

        callspec = getattr(item, "callspec", None)
        if callspec is None:
            continue

        scenario = callspec.params.get("run_scenario")
        if not callable(scenario):
            continue

        item.obj = _with_docstring(item.obj, inspect.getdoc(scenario))


def pytest_addoption(parser):
    parser.addoption(
        "--config",
        default="config.yaml",
        help="Path to the simulation YAML configuration",
    )


@pytest.fixture(scope="session")
def simulation_config(pytestconfig) -> Config:
    path = Path(pytestconfig.getoption("--config")).resolve()
    with path.open(encoding="utf-8") as file:
        data = yaml.safe_load(file)
        config = Config.model_validate(data or {})

    return config


@pytest.fixture(scope="function")
def simulator(simulation_config: Config) -> Generator[Simulator]:
    simulator = Simulator(simulation_config.simulator)

    try:
        yield simulator
    finally:
        simulator.close()


@pytest.fixture(scope="function")
def deterministic_simulator(
    simulation_config: Config,
) -> Generator[DeterministicSimulator]:
    simulator = DeterministicSimulator(simulation_config.simulator)

    try:
        yield simulator
    finally:
        simulator.close()
