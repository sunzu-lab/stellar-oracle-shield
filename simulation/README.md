# Oracle Shield simulation suite

This Python test framework exercises the compiled Oracle Shield contract through Stellar RPC. It simulates different inputs — operator reports, administrative configuration —  at differents levels of the Oracle Shield workflow and checks the scores, statuses, and errors visible to consumers.

```
 stellar       | Sun Zu Lab infra                         | stellar
---------------|------------------------------------------|-------------
L5             | L4            L3             L2          | L1       L0
 ↓             |  ↓             ↓              ↓          |  ↓        ↑
 [SDEX/CEX] →  |  [ingestion] → [indicators] → [scorer] → |  [contract]
```

* L2: indicators simulation - publish mid price
* L1: scorer simulation - set_score
* L0: contract result - get_score / get_status / event notification

The framework is composed of:
* a module `stellar_oracle_shield_simulation_suite` providing the simulation tools and the scenarios
* a generated module `stellar_oracle_shield_client` providing bindings to for oracle shield contract calls
* an application `generate-bindings` to generate it from a contract wasm
* a pytest core starting the different scenarios


## Prerequisites

- Python 3.10 or later.
- Docker with its daemon running for local stellar node and local scorer.
- [Poetry](https://python-poetry.org/docs/).
- Compiled contract Wasm. Local builds require Rust and the `wasm32v1-none` target.

## Quick start

From the repository root:

```bash
cargo build-wasm
cd simulation
poetry install
poetry run generate-bindings
./start-local-node
```

`poetry run generate-bindings` reads local WASM and does not require a running network. Generate it before pytest collection on a fresh checkout, and regenerate after every WASM change.

`start-local-node` starts an ephemeral `stellar-local` container with no persistent volume. Stopping it removes network state. On the next run, the simulator funds new identities and deploys contracts lazily as scenarios need them.

Check RPC readiness; repeat until `result.status` is `healthy`:

```bash
curl --fail --silent --show-error http://localhost:8000/rpc \
  -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

Run and stop:

```bash
poetry run pytest -v
./stop-local-node
```


## Configuration

The local Quickstart uses `simulation/config.yml` configuration.

Use `/rpc` consistently in `config.yaml` and local defaults. The network passphrase must match the node when signing transactions.

Pytest supports selecting a configuration:

```bash
poetry run pytest --config config.yaml -v
```

## Discovery and selection

```bash
# Discover
poetry run pytest --collect-only -v

# Run a subset
poetry run pytest -v -k quorum
```

## Writing a scenario

Contract scenarios live in `src/stellar_oracle_shield_simulation_suite/scenarios/contract/`.
Scenario extension is straighforward: just drop-in new files.
Name each module `scenario_*.py` and export `run(simulator)`.

For example, add `scenario_missing_pair.py`:

```python
from stellar_oracle_shield_simulation_suite.simulator import (
    DEFAULT_BASE,
    DEFAULT_QUOTE,
    OracleShieldError,
    Simulator,
    expect_contract_error,
)


def run(simulator: Simulator) -> None:
    """Reject reads for a pair with no operator report."""
    user = simulator.get_user()

    with expect_contract_error() as returned:
        user.get_score(DEFAULT_BASE, DEFAULT_QUOTE)

    assert returned.error == OracleShieldError.PairNotCovered, returned.error
```

Each pytest case receives a fresh simulator and, on first use, fresh accounts and a contract instance. New named operators are registered by default; `register=False` creates an unregistered publisher for negative scenarios.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Missing generated bindings | Run `poetry run generate-bindings` before pytest, including collection. |
| Missing WASM | Build it, check `wasm_path`, and run from `simulation/`. |
| RPC unavailable | Check `docker logs stellar-local`, the endpoint, and RPC health. |
| Container name exists | Check whether `stellar-local` is already running. |
| Authentication failure | Check network passphrase, signer, and configured roles. |

## Related documentation

- [Project overview and consensus rules](../README.md)
- [Threat model](../THREAT_MODEL.md)
- [Rust contract tests](../contracts/oracle-shield/src/contract/tests.rs)
