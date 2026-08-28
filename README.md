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

Each monitored `(base, quote)` pair has a score from **0 to 100**:

| Score  | Status     | Suggested interpretation                          |
| ------ | ---------- | ------------------------------------------------- |
| 66–100 | `Healthy`  | Oracle-dependent operations may proceed normally. |
| 33–65  | `Degraded` | Proceed only under stricter risk parameters.      |
| 0–32   | `Unsafe`   | Halt or reject oracle-dependent operations.       |

For example, a lending protocol could interpret `Degraded` by reducing maximum LTV, limiting operation size, or disabling particularly sensitive operations.

These are recommendations rather than protocol-enforced rules: Stellar Oracle Shield reports the condition, while the integrating protocol defines its own circuit-breaker policy.

### Stale and missing data

A status is returned only when a valid, sufficiently recent score exists for the requested pair.

Queries can fail when:

* the pair is not covered;
* the latest score is stale;
* required contract configuration is missing.

Consumers should normally treat unavailable health information as a **fail-closed condition** for safety-sensitive operations.

The default maximum score age is **3,600 seconds (1 hour)** unless changed by the administrator.

## Contract Roles

The contract uses two operational roles.

### Administrator

The administrator is configured when the contract is deployed.

The administrator can:

* set the operator address with `set_operator_key`;
* change the global maximum score age with `set_max_staleness`;
* upgrade the contract WASM.

### Operator

The operator is the account authorized to publish health scores using `set_score`.

The operator does **not** control protocol decisions. It only supplies the health score consumed by the contract.

### Protocol users

Reading a score or status does not require administrator or operator privileges. A Stellar smart contract can query the Shield before executing an oracle-dependent operation.

## Public Contract Interface

The main user-facing functions are:

```text
get_status(base, quote) -> Result<Status, Error>
get_score(base, quote)  -> Result<u32, Error>
version()                -> (u32, u32, u32)
```

Administrative/operator functions are:

```text
set_score(base, quote, score)
set_operator_key(operator_key)
set_max_staleness(max_staleness)
upgrade(new_wasm_hash)
```

`base` and `quote` are Stellar Asset Contract (SAC) addresses.

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
  set_operator_key \
  --operator_key <OPERATOR_ADDRESS>
```

Optionally change the default maximum staleness:

```bash
./invoke testnet alice \
  set_max_staleness \
  --max_staleness 900
```

The value is expressed in seconds. For example, `900` means that a score older than 15 minutes is considered stale.

## Publishing a Health Score

Only the configured operator can call `set_score`.

Assuming a Stellar CLI identity named `operator` owns the configured operator address:

```bash
./invoke testnet operator \
  set_score \
  --base <BASE_SAC_ADDRESS> \
  --quote <QUOTE_SAC_ADDRESS> \
  --score 82
```

The score must be between `0` and `100`.

When a new score crosses a health-status boundary, the contract publishes a `StatusChange` event containing the base asset, quote asset, and new status.

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

To retrieve the underlying 0–100 score:

```bash
./invoke testnet alice \
  get_score \
  --base <BASE_SAC_ADDRESS> \
  --quote <QUOTE_SAC_ADDRESS>
```

A query will return an error instead of a score/status when the pair is not covered or its latest input is stale.

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

The safest default is also to reject or restrict the operation if the Shield call fails because the pair is stale or not covered.

## Errors

The contract currently defines the following errors:

| Code | Error               | Meaning                                                                 |
| ---: | ------------------- | ----------------------------------------------------------------------- |
|  701 | `MissingAdmin`      | Administrator configuration is missing.                                 |
|  702 | `ScoreBounds`       | A score outside the 0–100 range was supplied.                           |
|  703 | `PairNotCovered`    | No usable score exists for the requested pair.                          |
|  704 | `StaleInput`        | The pair's latest score is older than the configured maximum staleness. |
|  705 | `ConversionError`   | Internal value conversion failed.                                       |
|  706 | `NoMaxStalenessSet` | Maximum staleness configuration is missing.                             |
|  707 | `MissingOperator`   | No operator has been configured.                                        |

For integrations guarding financial operations, `PairNotCovered` and `StaleInput` should normally be handled conservatively rather than ignored.

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
* the Shield score is stale;
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
* [Rust client API documentation](https://docs.rs/stellar-oracle-shield-client)
* [Stellar smart contract documentation](https://developers.stellar.org/docs/build/smart-contracts)
