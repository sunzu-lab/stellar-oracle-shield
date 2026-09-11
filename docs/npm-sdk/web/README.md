# Oracle Shield - minimal web example

This Vite app demonstrates how to query an Oracle Shield contract, subscribe to
status changes, and let an authorized operator publish a score from a browser
wallet.

`base` and `quote` must be Stellar Asset Contract (SAC) addresses, not asset
codes such as `XLM` or `USDC`.

## 1. Authenticate to GitHub Packages

The package lives on GitHub Packages, which requires a token **even to read**.
Create a token with the `read:packages` scope and export it:

```bash
export GITHUB_TOKEN=ghp_xxx
```

The [.npmrc](.npmrc) in this folder routes the `@sunzulab` scope to GitHub
Packages and picks up that token.

## 2. Install & run

```bash
cd docs/npm-sdk/web
npm install
npm run dev
```

Open the local URL printed by Vite (by default, `http://localhost:5173`).

The example is configured for Stellar Testnet. Enter the ID of a deployed
Oracle Shield contract and a covered pair in the form.

## 3. Reading: `get_status` and `get_score`

These are two read-only functions. Call whichever you need, you don't have to call both:

- `get_status` → the **verdict**: `Healthy` / `Degraded` / `Unsafe`.
- `get_score` → the **raw number** `0–100`.

```js
import { Client } from "@sunzulab/oracle-shield-ts-sdk";

const client = new Client({
  contractId: "C...",                              // your deployed contract
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
});

// The verdict:
const status = await client.get_status({ base, quote });
status.result.unwrap(); // { tag: "Healthy" | "Degraded" | "Unsafe" }

// The score:
const score = await client.get_score({ base, quote });
score.result.unwrap();  // 0–100
```

## 4. Subscribing to `status_change`

Choose **Subscribe Status Change**, enter a contract and pair, and click
**Subscribe**. The example polls Stellar RPC's `getEvents` method every five
seconds and appends notifications when that pair crosses a health-status
boundary. Click **Stop subscription** or switch modes to stop polling.

The subscription starts at the latest ledger, so it displays new events rather
than replaying the RPC server's retained event history. This is a lightweight
browser example; production applications should persist their cursor and
deduplicate events by event ID.

## 5. Writing: `set_score` (operator only)

`set_score` changes state, so it must be **signed** and submitted. Signing goes
through a **wallet** ([Stellar Wallets Kit](https://github.com/Creit-Tech/Stellar-Wallets-Kit))
LOBSTR, Freighter, WalletConnect, Ledger… So the **secret key never touches
the page**:

```js
import { StellarWalletsKit } from "@creit.tech/stellar-wallets-kit/sdk";
import { defaultModules } from "@creit.tech/stellar-wallets-kit/modules/utils";
import { Client } from "@sunzulab/oracle-shield-ts-sdk";

StellarWalletsKit.init({ modules: defaultModules() });
const { address } = await StellarWalletsKit.authModal();   // wallet picker; public key only

const client = new Client({
  contractId, rpcUrl, networkPassphrase,
  publicKey: address,
  signTransaction: (xdr) => StellarWalletsKit.signTransaction(xdr, { address, networkPassphrase }),
  signAuthEntry:   (xdr) => StellarWalletsKit.signAuthEntry(xdr, { address, networkPassphrase }),
});

const tx = await client.set_score({ base, quote, score });
await tx.signAndSend();                            // wallet prompts to approve
```

> The connected wallet must be the contract's configured **operator**. The
> administrator configures that address with `set_operator_key`; otherwise the
> call fails with `MissingOperator` or an authorization error. See
> [Publishing a Health Score](../../../README.md#publishing-a-health-score).

## Generating the client yourself (from a live contract)

Instead of depending on the published package, a downstream integrator can
generate a client from a deployed contract without GitHub Packages credentials:

```bash
# Reads the contract interface directly from chain
npx @stellar/stellar-sdk@16.2.0 generate \
  --contract-id C... \
  --network testnet \
  --output-dir src/oracle-shield-sdk --overwrite
```

The generated client reflects the interface of the contract from which it was
generated. Import it from the output directory and point it at that contract's
network.

## Notes

- This example needs a contract **already deployed** on Testnet, and the pair must
  be **covered** by the oracle (otherwise it returns `PairNotCovered`).
- The repository's current public Testnet contract ID is listed in
  [Deployed Contracts](../../../README.md#deployed-contracts). Testnet is for
  development and integration testing only.
- The SDK depends on `@stellar/stellar-sdk`; a bundler (here Vite) handles the
  browser build for you.
