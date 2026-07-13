const NETWORK = {
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
  explorer: "https://stellar.expert/explorer/testnet",
};

const $ = (id) => document.getElementById(id);

const Mode = Object.freeze({ GET: "get", SET: "set" });
let mode = Mode.GET;

function setMode(next) {
  mode = next;
  $("modeGet").classList.toggle("active", mode === Mode.GET);
  $("modeSet").classList.toggle("active", mode === Mode.SET);
  $("setFields").hidden = mode !== Mode.SET;
  $("run").textContent = mode === Mode.GET ? "Query" : "Submit";
  $("out").textContent = "Result appears here…"; 
}

$("modeGet").addEventListener("click", () => setMode(Mode.GET));
$("modeSet").addEventListener("click", () => setMode(Mode.SET));

let kit = null;
let walletAddress = null;

async function getKit() {
  if (kit) return kit;

  const { StellarWalletsKit } = await import("@creit.tech/stellar-wallets-kit/sdk");
  const { defaultModules } = await import("@creit.tech/stellar-wallets-kit/modules/utils");

  StellarWalletsKit.init({ modules: defaultModules() });

  kit = StellarWalletsKit;
  return kit;
}

$("connect").addEventListener("click", async () => {
  try {
    const k = await getKit();
    // Opens the wallet-picker modal and resolves with the connected address.
    const { address } = await k.authModal();
    walletAddress = address;
    $("walletAddr").textContent = address;
  } catch (e) {
    $("walletAddr").textContent = "Error: " + (e?.message ?? e);
  }
});

$("run").addEventListener("click", async () => {
  const out = $("out");
  const contractId = $("contract").value.trim();
  const pair = { base: $("base").value.trim(), quote: $("quote").value.trim() };

  if (!contractId || !pair.base || !pair.quote) {
    out.textContent = "Fill in Contract ID, Base and Quote.";
    return;
  }

  $("run").disabled = true;

  try {
    const { Client } = await import("@sunzulab/oracle-shield-ts-sdk");

    if (mode === Mode.GET) {
      out.textContent = "Querying…";

      const client = new Client({
        contractId,
        rpcUrl: NETWORK.rpcUrl,
        networkPassphrase: NETWORK.networkPassphrase,
      });

      const [status, score] = await Promise.all([
        client.get_status(pair),
        client.get_score(pair),
      ]);

      out.textContent = JSON.stringify(
        { status: status.result.unwrap(), score: score.result.unwrap() },
        null,
        2,
      );

    } else {
      if (!walletAddress) {
        out.textContent = "Connect a wallet first.";
        return;
      }

      const raw = $("score").value.trim();
      const score = Number(raw);

      if (raw === "" || !Number.isInteger(score) || score < 0 || score > 100) {
        out.textContent = "Score must be an integer between 0 and 100.";
        return;
      }

      out.textContent = "Waiting for wallet signature…";

      const k = await getKit();
      const client = new Client({
        contractId,
        rpcUrl: NETWORK.rpcUrl,
        networkPassphrase: NETWORK.networkPassphrase,
        publicKey: walletAddress,
        signTransaction: (xdr) =>
          k.signTransaction(xdr, { address: walletAddress, networkPassphrase: NETWORK.networkPassphrase }),
        signAuthEntry: (xdr) =>
          k.signAuthEntry(xdr, { address: walletAddress, networkPassphrase: NETWORK.networkPassphrase }),
      });
      const tx = await client.set_score({ ...pair, score });
      const sent = await tx.signAndSend();
      const hash = sent?.sendTransactionResponse?.hash;

      out.textContent = "✅ Score set." + (hash ? `\ntx: ${NETWORK.explorer}/tx/${hash}` : "");
    }
  } catch (e) {
    out.textContent = "Error: " + (e?.message ?? e);
  } finally {
    $("run").disabled = false;
  }
});
