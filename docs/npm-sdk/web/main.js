const NETWORK = {
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
  explorer: "https://stellar.expert/explorer/testnet",
};

const $ = (id) => document.getElementById(id);

const Mode = Object.freeze({ GET: "get", SUBSCRIBE: "subscribe", SET: "set" });
let mode = Mode.GET;
let subscriptionId = 0;

const sleep = (milliseconds) =>
  new Promise((resolve) => setTimeout(resolve, milliseconds));

function stopSubscription(message) {
  subscriptionId += 1;
  if (message) $("out").textContent = message;
}

function setMode(next) {
  if (mode === Mode.SUBSCRIBE && next !== Mode.SUBSCRIBE) {
    stopSubscription();
  }
  mode = next;
  $("modeGet").classList.toggle("active", mode === Mode.GET);
  $("modeSubscribe").classList.toggle("active", mode === Mode.SUBSCRIBE);
  $("modeSet").classList.toggle("active", mode === Mode.SET);
  $("setFields").hidden = mode !== Mode.SET;
  $("run").textContent =
    mode === Mode.GET ? "Query" : mode === Mode.SUBSCRIBE ? "Subscribe" : "Submit";
  $("out").textContent = "Result appears here…"; 
}

$("modeGet").addEventListener("click", () => setMode(Mode.GET));
$("modeSubscribe").addEventListener("click", () => setMode(Mode.SUBSCRIBE));
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

async function subscribeToStatusChanges(contractId, pair, out) {
  const { rpc, scValToNative } = await import("@stellar/stellar-sdk");
  const server = new rpc.Server(NETWORK.rpcUrl);
  const latestLedger = await server.getLatestLedger();
  const id = ++subscriptionId;
  let cursor;

  out.textContent = `Listening from ledger ${latestLedger.sequence}…`;
  $("run").textContent = "Stop subscription";
  $("run").disabled = false;

  while (id === subscriptionId) {
    const request = {
      filters: [{ type: "contract", contractIds: [contractId] }],
      limit: 100,
      ...(cursor ? { cursor } : { startLedger: latestLedger.sequence }),
    };
    const response = await server.getEvents(request);
    cursor = response.cursor;

    for (const event of response.events) {
      const topics = event.topic.map(scValToNative);
      const [name, base, quote] = topics;

      if (name !== "status_change" || base !== pair.base || quote !== pair.quote) {
        continue;
      }

      const decodedStatus = scValToNative(event.value);
      const notification = {
        status: decodedStatus?.tag ?? (Array.isArray(decodedStatus) ? decodedStatus[0] : decodedStatus),
        ledger: event.ledger,
        ledgerClosedAt: event.ledgerClosedAt,
        txHash: event.txHash,
      };
      out.textContent += `\n\n${JSON.stringify(notification, null, 2)}`;
      out.scrollTop = out.scrollHeight;
    }

    await sleep(5000);
  }
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

  if (mode === Mode.SUBSCRIBE && $("run").textContent === "Stop subscription") {
    stopSubscription("Subscription stopped.");
    $("run").textContent = "Subscribe";
    return;
  }

  $("run").disabled = true;

  try {
    if (mode === Mode.GET) {
      out.textContent = "Querying…";

      const { Client } = await import("@sunzu-lab/oracle-shield-ts-sdk");
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

    } else if (mode === Mode.SUBSCRIBE) {
      await subscribeToStatusChanges(contractId, pair, out);
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
      const { Client } = await import("@sunzu-lab/oracle-shield-ts-sdk");
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
    if (mode === Mode.SUBSCRIBE) {
      stopSubscription();
      $("run").textContent = "Subscribe";
    }
    $("run").disabled = false;
  }
});
