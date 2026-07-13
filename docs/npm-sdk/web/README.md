# Oracle Shield — minimal web example

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
npm install
npm run dev
```

Open the Website
## 3. Reading: `get_status` and `get_score`

These are **two independent** read-only functions on the same pair. Call
whichever you need — you don't have to call both:

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

## 4. Writing: `set_score` (admin only)

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

> The connected wallet must be the contract's **admin** (set at deploy via
> `--admin`), otherwise `require_auth(admin)` fails.

## Generating the client yourself (from a live contract)

Instead of depending on the published package (steps 1–2), a downstream
integrator can generate the same client straight from **any deployed contract**
— no token, no npm scope needed:

```bash
# Reads the contract interface directly from chain:
stellar contract bindings typescript \
  --contract-id C... \
  --network testnet \
  --output-dir src/oracle-shield-sdk --overwrite
```

The generated `Client` is identical to the published one. Point it at whichever contract and network you
integrate against.

## Notes

- Needs a contract **already deployed** on the target network, and the pair must
  be **covered** by the oracle (otherwise it returns `PairNotCovered`).
- For mainnet, use `rpcUrl: "https://mainnet.sorobanrpc.com"` and
  `networkPassphrase: "Public Global Stellar Network ; September 2015"`.
- The SDK depends on `@stellar/stellar-sdk`; a bundler (here Vite) handles the
  browser build for you.
