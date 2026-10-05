# Stellar Oracle Shield

Stellar Oracle Shield is an oracle risk monitoring and circuit-breaker infrastructure layer for Stellar Smart Contract DeFi. It does not replace price feeds. Instead, it helps protocols decide whether current market conditions are safe enough to trust and act on oracle prices.

## Open-Source Oracle Health Smart Contract

A Stellar smart contract that protocols can query before accepting oracle-dependent transactions such as swaps, liquidations, collateral valuation, minting, or redemption operations. The contract exposes a simple health interface returning a status such as healthy, degraded, or unsafe.

Stellar integration: Stellar smart contract, deployed first on testnet and then mainnet, callable by any Stellar DeFi protocol.

## How Stellar Oracle Shield Works

Stellar Oracle Shield is a **risk signal**, not a price oracle.

A protocol continues to obtain asset prices from its normal oracle or price-feed infrastructure. Before executing an oracle-dependent operation, it can query Stellar Oracle Shield for the health of the corresponding asset pair.

Typical protected operations include:

* swaps;
* liquidations;
* collateral valuation;
* borrowing and lending;
* minting and redemption;
* any operation whose safety depends on a reliable market price.

The intended flow is:

```text
Price oracle / market data
          │
          ▼
   Risk monitoring
          │
          ▼
 Stellar Oracle Shield
          │
          ├── Healthy
          ├── Degraded
          └── Unsafe
          │
          ▼
    DeFi protocol
```

The consuming protocol remains responsible for deciding what action to take for each status.

## Health Model

Each monitored ordered `(base, quote)` pair has an aggregated score from **0 to 100**. The contract computes this score from fresh operator reports using the consensus rules below, then maps it to a status:

| Score  | Status     | Suggested interpretation                          |
| ------ | ---------- | ------------------------------------------------- |
| 66–100 | `Healthy`  | Oracle-dependent operations may proceed normally. |
| 33–65  | `Degraded` | Proceed only under stricter risk parameters.      |
| 0–32   | `Unsafe`   | Halt or reject oracle-dependent operations.       |

For example, a lending protocol could interpret `Degraded` by reducing maximum LTV, limiting operation size, or disabling particularly sensitive operations.

These are recommendations rather than protocol-enforced rules: Stellar Oracle Shield reports the condition, while the integrating protocol defines its own circuit-breaker policy.

### Scoring consensus

Each registered operator publishes its own score for an ordered `(base, quote)` pair. The contract keeps only that operator's latest report for the pair, timestamped with the ledger time. Publishing again replaces the report; it does not add another vote. Reports for `(quote, base)` are separate.

`get_score` and `get_status` now compute consensus from the current reports on every query:

1. Collect the latest reports from **currently registered operators**. Ignore missing reports and reports whose age exceeds `max_staleness`.
2. Sort all fresh scores and calculate their **median**. With an even number of reports, use the average of the two middle values, rounded down to an integer.
3. Count the fresh reports satisfying `abs(report_score - median) <= max_deviation`.
4. Return the median score, or its corresponding health status, only if this count is at least `quorum`. Otherwise, return `QuorumNotReached`.

Agreement is based on **numeric distance in score points**. All fresh reports participate in the median calculation, including those outside the deviation limit. The deviation check gates availability; it does not remove outliers and calculate another median.

| Setting | Default | Meaning |
| ------- | ------- | ------- |
| `max_staleness` | `3600` | Maximum report age in seconds, inclusive. |
| `max_deviation` | `5` | Maximum absolute distance from the median in score points, inclusive. |
| `quorum` | `1` | Minimum number of fresh reports within the deviation limit; an absolute count, not a percentage or automatic majority. |

These defaults apply when constructor options are omitted and when the corresponding stored settings are absent after an upgrade. Administrators can change them with `set_max_staleness`, `set_max_deviation`, and `set_quorum`.

A quorum of `0` disables the agreement-count requirement, but at least one fresh report is still required. The default quorum of `1` allows a single fresh operator report to produce a result. Configure a higher quorum to require multiple agreeing operators; a quorum greater than the number of registered operators cannot be reached.

For example, with `max_deviation = 5`:

| Fresh scores | Median | Agreeing reports | Result with `quorum = 2` |
| ------------ | ------ | ---------------- | ------------------------ |
| `10, 70, 74` | `70` | `2` (`70`, `74`) | `70`, `Healthy` |
| `70, 74` | `72` | `2` | `72`, `Healthy` |
| `32, 33` | `32` | `2` | `32`, `Unsafe` |
| `0, 100` | `50` | `0` | `QuorumNotReached` |

The last example also fails with the default quorum of `1`: neither report is within five points of the median. With quorum `0`, it returns `50` (`Degraded`).

Freshness, operator membership, and consensus settings are evaluated on every query. A score or status can therefore change, or become unavailable, without a new publication. Removing an operator immediately excludes its reports from subsequent calculations.

### Stale and missing data

Queries return an error instead of a score or status when:

* no stored report exists for the pair from any currently registered operator (`PairNotCovered`);
* reports exist for currently registered operators, but none is fresh and eligible (`StaleInput`);
* fresh reports exist, but too few agree with the median (`QuorumNotReached`).

Reports are kept in temporary contract storage. If all relevant reports have expired from storage, the result is `PairNotCovered` rather than `StaleInput`.

Consumers should normally treat unavailable health information as a **fail-closed condition** for safety-sensitive operations. A failed consensus query does not return the last successful score or an `Unsafe` status.

## Contract Roles

The contract uses two operational roles.

### Administrator

The administrator is configured when the contract is deployed.

The administrator can:

* replace the allowed operator set with `set_operators`, or update it with `add_operator` and `remove_operator`;
* change the global maximum report age with `set_max_staleness`;
* configure score agreement with `set_max_deviation` and `set_quorum`;
* upgrade the contract WASM.
* hand over its privileges

### Operators

Operators are accounts authorized to publish health scores using `set_score_from`.

Operators do **not** control protocol decisions. They supply individual health reports from which the contract computes the consensus score.

### Protocol users

Reading a score or status does not require administrator or operator privileges. A Stellar smart contract can query the Shield before executing an oracle-dependent operation.

## Public Contract Interface

The main user-facing functions are:

```text
get_status(base, quote) -> Result<Status, Error>
get_score(base, quote)  -> Result<u32, Error>
version()                -> (u32, u32, u32)
```

Administrative functions are:

```text
set_operators(operators)
add_operator(operator)
remove_operator(operator)
set_max_staleness(max_staleness)
set_max_deviation(max_deviation)
set_quorum(quorum)
upgrade(new_wasm_hash)
hand_over_admin(new_admin)
accept_admin()
```

Operator functions are:

```text
set_score_from(operator, base, quote, score)
```

`base` and `quote` are Stellar Asset Contract (SAC) addresses. The public getters still return only the aggregated numeric score or status; they do not expose individual reports or the agreement count.

### Status values

```rust
pub enum Status {
    Healthy,
    Degraded,
    Unsafe,
}
```

## Deployed Contracts

A public instance of Stellar Oracle Shield is currently deployed on the **Stellar Testnet** and is available for development and integration testing.

| Network         | Contract ID                                                | Status |
| --------------- | ---------------------------------------------------------- | ------ |
| Stellar Testnet | `CCMSDPGXS3VMCQCGUJDIEY6UPJUGO5GBPWWKIWZUPZA7GWRWKIITYE7P` | Active |

You can register the deployed contract locally with the Stellar CLI:

```bash
stellar contract alias set \
  stellar_oracle_shield \
  --id CCMSDPGXS3VMCQCGUJDIEY6UPJUGO5GBPWWKIWZUPZA7GWRWKIITYE7P \
  --network testnet
```

You can then interact with it using the repository's `invoke` helper. For example:

```bash
./invoke testnet <IDENTITY> version
```

For on-chain integrations, use the contract ID above as the `shield_address` when creating a Stellar Oracle Shield client.

> **Testnet notice:** This deployment is intended for development, testing, and demonstration purposes. Do not assume testnet configuration, data availability, operator behavior, or contract state is suitable for production use.

## Quick Start

### Prerequisites

You need:

* Rust with the Stellar/Soroban target configured;
* Stellar CLI;
* a funded Stellar account for the network you want to use.

Clone the repository:

```bash
git clone https://github.com/sunzu-lab/stellar-oracle-shield.git
cd stellar-oracle-shield
```

Run the tests:

```bash
cargo test
```

Build the contracts:

```bash
stellar contract build --optimize
```

The main contract WASM is generated at:

```text
target/wasm32v1-none/release/stellar_oracle_shield.wasm
```

## Deploying a Shield Instance

The repository contains a `deploy` helper.

For example, using a Stellar CLI identity called `alice` on testnet:

```bash
./deploy testnet alice
```

The deployment account becomes the contract administrator.

The helper also creates the local contract alias:

```text
stellar_oracle_shield
```

> **Important:** the default deployment helper does not configure an operator. An administrator must set one before scores can be published.

Configure the operator:

```bash
./invoke testnet alice \
  add_operator \
  --operator <OPERATOR_ADDRESS>
```

Optionally change the default maximum staleness:

```bash
./invoke testnet alice \
  set_max_staleness \
  --max_staleness 900
```

The value is expressed in seconds. For example, `900` excludes reports older than 15 minutes.

For a setup requiring two agreeing operators, register at least two operator addresses and configure consensus:

```bash
./invoke testnet alice \
  set_max_deviation \
  --max_deviation 5

./invoke testnet alice \
  set_quorum \
  --quorum 2
```

Each operator must publish its own report for each monitored pair. Reads fail with `QuorumNotReached` until enough fresh reports agree with the median.

## Publishing a Health Score

Each registered operator can call `set_score_from`, authenticating as the address passed in `--operator`.

Assuming a Stellar CLI identity named `operator` owns a registered operator address:

```bash
./invoke testnet operator \
  set_score_from \
  --operator <OPERATOR_ADDRESS> \
  --base <BASE_SAC_ADDRESS> \
  --quote <QUOTE_SAC_ADDRESS> \
  --score 82
```

The score must be between `0` and `100`.

An authorized publication of a valid score succeeds even when consensus is not yet available. The report is stored, and later publications can establish quorum.

### Consensus events

After each publication, the contract compares the aggregate before and after the update:

* `ScoreChange` (`score_change`) is published when the new aggregate is available and its numeric value changes. It contains the base asset, quote asset, and new aggregate score.
* `StatusChange` (`status_change`) is published when the new aggregate is available and its health status changes. It contains the base asset, quote asset, and new status.
* Both events are published when an aggregate first becomes available, or becomes available again after the previous aggregate was unavailable.

No event is published if the new aggregate is unavailable. These events are emitted by score publications, not by queries, passage of time, or administrative changes. Consumers must query current health rather than rely on events alone to detect lost quorum or stale reports.

## Querying Oracle Health

### Get the status

Anyone can query the status of a covered pair:

```bash
./invoke testnet alice \
  get_status \
  --base <BASE_SAC_ADDRESS> \
  --quote <QUOTE_SAC_ADDRESS>
```

The result is one of:

```text
Healthy
Degraded
Unsafe
```

### Get the numeric score

To retrieve the current aggregated 0–100 score:

```bash
./invoke testnet alice \
  get_score \
  --base <BASE_SAC_ADDRESS> \
  --quote <QUOTE_SAC_ADDRESS>
```

Both getters use the same freshness and consensus checks. They return an error when the pair has no eligible reports or the configured quorum is not reached.

## Integrating From Another Stellar Smart Contract

For on-chain integrations, use the published Rust client crate:

```toml
[dependencies]
stellar-oracle-shield-client = "0.1.3"
```

The client exposes the Shield contract interface and the `Status` and `Error` types.

A minimal integration looks like:

```rust
use stellar_oracle_shield_client::{ContractClient, Status};
use soroban_sdk::{Address, Env};

fn oracle_operation_is_allowed(
    env: &Env,
    shield_address: &Address,
    base: &Address,
    quote: &Address,
) -> bool {
    let shield = ContractClient::new(env, shield_address);

    match shield.get_status(base, quote) {
        Status::Healthy => true,
        Status::Degraded => false, // replace with your protocol policy
        Status::Unsafe => false,
    }
}
```

In a production protocol, prefer the `try_*` client methods where you need to distinguish contract errors such as stale or uncovered pairs from invocation/conversion failures.

An end-to-end example is available in:

```text
contracts/oracle-shield-client-example/
```

### Recommended integration pattern

A protocol should define its policy explicitly rather than treating the health status as a generic boolean.

For example:

```rust
match shield_status {
    Status::Healthy => {
        // Normal protocol parameters.
    }
    Status::Degraded => {
        // Reduce limits, tighten LTV, restrict trade size,
        // or disable selected risk-sensitive operations.
    }
    Status::Unsafe => {
        // Reject the oracle-dependent operation.
    }
}
```

The safest default is also to reject or restrict the operation if the Shield call fails because the pair is stale, not covered, or lacks quorum.

## Errors

The contract currently defines the following errors:

| Code | Error | Meaning |
| ---: | ----- | ------- |
| 701 | `MissingAdmin` | Administrator configuration is missing. |
| 702 | `ScoreBounds` | A score outside the 0–100 range was supplied. |
| 703 | `PairNotCovered` | No stored report exists for the pair from a currently registered operator. |
| 704 | `StaleInput` | Reports exist for registered operators, but none is fresh and eligible. |
| 705 | `ConversionError` | Internal value conversion failed. |
| 706 | `NoMaxStalenessSet` | Retained in the error enum for compatibility; consensus reads use the default when this setting is absent. |
| 707 | `UnauthorizedOperator` | The publisher is not registered |
| 708 | `QuorumNotReached` | Fresh reports exist, but fewer than `quorum` are within `max_deviation` points of the median. |

For integrations guarding financial operations, `PairNotCovered`, `StaleInput`, and `QuorumNotReached` should normally be handled conservatively rather than ignored.

## Upgrading the Contract

The repository includes an `upgrade` helper:

```bash
stellar contract build --optimize
./upgrade testnet alice
```

The upgrade must be authorized by the contract administrator.

The helper uploads the newly built WASM and invokes the existing contract's `upgrade` function with the new WASM hash.

You can query the deployed contract version with:

```bash
./invoke testnet alice version
```

## Security and Integration Considerations

Stellar Oracle Shield is an additional risk-control layer. It does not:

* provide asset prices itself;
* guarantee that an external price oracle is correct;
* automatically pause a consuming protocol;
* replace protocol-specific risk management.

Integrating protocols should define how `Healthy`, `Degraded`, unavailable and `Unsafe` states affect every oracle-dependent operation.

In particular, consider adopting a fail-closed policy when:

* the pair is not covered;
* no fresh operator report remains;
* the configured consensus quorum is not reached;
* invocation of the Shield fails unexpectedly.

Before using the system in production, independently review the smart contract, operator architecture, score-generation methodology, deployment configuration and upgrade controls.

## Repository Structure

```text
contracts/
├── oracle-shield/                 # Main Oracle Shield Soroban contract
├── oracle-shield-client/          # Reusable on-chain Rust client/interface
└── oracle-shield-client-example/  # Example consuming contract

deploy                             # Deployment helper
invoke                             # Contract invocation helper
upgrade                            # WASM upgrade helper
docs/                              # Generated SDK/API documentation
```

## Further Documentation

* [`contracts/oracle-shield-client/README.md`](contracts/oracle-shield-client/README.md) — on-chain Rust integration example
* [`contracts/oracle-shield-client-example/`](contracts/oracle-shield-client-example/) — working example contract
* [Threat Model](THREAT_MODEL.md)
* [Rust client API documentation](https://docs.rs/stellar-oracle-shield-client)
* [Stellar smart contract documentation](https://developers.stellar.org/docs/build/smart-contracts)
