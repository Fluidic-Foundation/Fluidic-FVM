"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function TypeScriptSdkPage() {
  return (
    <DocPage title="TypeScript SDK">
      <p>
        The TypeScript SDK provides keypair generation, a typed HTTP client, and helpers for building and submitting shifts.
      </p>

      <h2>Installation</h2>
      <pre><code>{`npm install @fluidic/sdk
# or link from the repo
npm link /path/to/fluidic/sdk/typescript`}</code></pre>

      <h2>FluidicClient</h2>
      <pre><code>{`const client = new FluidicClient({
  apiUrl: "http://localhost:8080",
});`}</code></pre>

      <h2>Key methods</h2>
      <ul>
        <li><code>register(publicKeyHex)</code> — register an account and faucet seed.</li>
        <li><code>submitStateful(shift)</code> — submit a signed stateful shift.</li>
        <li><code>submitCommutative(shift)</code> — submit a commutative delta.</li>
        <li><code>submitEvm(tx)</code> — submit a raw EVM transaction.</li>
        <li><code>getState()</code> — fetch live pool state.</li>
        <li><code>waitForStatus(hash, status, opts)</code> — poll finality.</li>
      </ul>

      <h2>Keypairs</h2>
      <pre><code>{`import { FluidicKeypair } from "@fluidic/sdk";

const wallet = FluidicKeypair.generate();
console.log("account:", wallet.accountId);
console.log("public key:", wallet.publicKeyHex);`}</code></pre>

      <h2>Building shifts</h2>
      <pre><code>{`import { buildStatefulShift, buildCommutativeShift } from "@fluidic/sdk";

const stateful = buildStatefulShift({
  signer: wallet,
  to: recipientAccountId,
  amount: 1_000_000n,
  vectorClock: { entries: { [wallet.accountId]: 1n } },
  nonce: 0n,
});

const commutative = buildCommutativeShift({
  signer: wallet,
  domain: DEFAULT_DEX_DOMAIN,
  waveDelta: -1_000_000n,
  usdcDelta: 990_000n,
  nonce: 1n,
});`}</code></pre>

      <h2>Submit a swap</h2>
      <pre><code>{`import { submitSwap } from "@fluidic/sdk";

const { poolInHash } = await submitSwap(client, {
  signer: wallet,
  direction: "WAVE_TO_USDC",
  amount: 1_000_000n,
});

const status = await client.waitForStatus(poolInHash, "finalized", {
  timeoutMs: 30_000,
});`}</code></pre>

      <h2>Submit an EVM transaction</h2>
      <pre><code>{`const txHash = await client.submitEvm({
  from: "0x...",
  to: contractAddress,
  data: calldata,
  value: 0n,
  gas_limit: 100000n,
  gas_price: 1n,
  nonce: 0,
  chain_id: 990492,
  v: 0n,
  r: "0x...",
  s: "0x...",
});`}</code></pre>

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/tutorials/build-dapp">Build a dApp</Link> — React example using the SDK.</li>
        <li><Link href="/docs/core-concepts/accounts">Accounts</Link> — how keys map to AccountIds.</li>
        <li><Link href="/docs/api-reference/rest-api">REST API</Link> — endpoints the client wraps.</li>
      </ul>
    </DocPage>
  );
}
