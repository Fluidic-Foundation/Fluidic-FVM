"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function BuildDappPage() {
  return (
    <DocPage title="Tutorial: Build a dApp">
      <p>
        This tutorial builds a minimal React dApp that creates a Fluidic wallet, registers it with a node, and submits a commutative swap. You can extend the same pattern to read contract state through the EVM RPC or listen to live state over WebSocket.
      </p>

      <h2>Prerequisites</h2>
      <ul>
        <li>Node.js 18+ and a React project (Vite or Next.js).</li>
        <li>A running Fluidic node or access to <code>https://api.testnet.fluidic.foundation</code>.</li>
      </ul>

      <h2>1. Install the SDK</h2>
      <pre><code>{`npm install @fluidic-foundation/sdk
# or link from the repo
npm link /path/to/fluidic/sdk/typescript`}</code></pre>

      <h2>2. Create a wallet and register</h2>
      <p>
        Generate an Ed25519 keypair, then register the public key with the node. Registration seeds the account with test tokens.
      </p>
      <pre><code>{`import { FluidicClient, FluidicKeypair } from "@fluidic-foundation/sdk";

const client = new FluidicClient({
  apiUrl: "https://api.testnet.fluidic.foundation",
});

const wallet = FluidicKeypair.generate();
await client.register(wallet.publicKeyHex);

console.log("account:", wallet.accountId);`}</code></pre>

      <h2>3. Submit a swap</h2>
      <pre><code>{`import { submitSwap } from "@fluidic-foundation/sdk";

const { poolInHash } = await submitSwap(client, {
  signer: wallet,
  direction: "WAVE_TO_USDC",
  amount: 1_000_000n,
});

console.log("swap submitted:", poolInHash);

// Poll until finalized
const status = await client.waitForStatus(poolInHash, "finalized", {
  timeoutMs: 30_000,
});
console.log("status:", status);`}</code></pre>

      <h2>4. Wire it to a React component</h2>
      <pre><code>{`import { useEffect, useState } from "react";
import { FluidicClient, FluidicKeypair, submitSwap } from "@fluidic-foundation/sdk";

const client = new FluidicClient({
  apiUrl: "https://api.testnet.fluidic.foundation",
});

export function SwapCard() {
  const [wallet, setWallet] = useState(null);
  const [status, setStatus] = useState("");

  useEffect(() => {
    const w = FluidicKeypair.generate();
    client.register(w.publicKeyHex).then(() => setWallet(w));
  }, []);

  async function swap() {
    if (!wallet) return;
    setStatus("submitting");
    const { poolInHash } = await submitSwap(client, {
      signer: wallet,
      direction: "WAVE_TO_USDC",
      amount: 1_000_000n,
    });
    await client.waitForStatus(poolInHash, "finalized", { timeoutMs: 30_000 });
    setStatus("finalized: " + poolInHash);
  }

  return (
    <div>
      <p>Account: {wallet?.accountId ?? "loading..."}</p>
      <button onClick={swap} disabled={!wallet}>Swap WAVE → USDC</button>
      <p>{status}</p>
    </div>
  );
}`}</code></pre>

      <h2>5. Read EVM contract state</h2>
      <p>
        If your dApp also uses Solidity contracts, read their state through the standard EVM RPC.
      </p>
      <pre><code>{`const response = await fetch("https://api.testnet.fluidic.foundation/rpc", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: "eth_call",
    params: [
      { to: "0xC89Ce4735882C9F0f0FE26686c53074E09B0D550", data: "0x06661abd" },
      "latest",
    ],
  }),
});
const { result } = await response.json();
console.log("count:", BigInt(result));`}</code></pre>

      <h2>Next steps</h2>
      <ul>
        <li><Link href="/docs/api-reference/typescript-sdk">TypeScript SDK reference</Link> — all client methods.</li>
        <li><Link href="/docs/api-reference/rest-api">REST API</Link> — polling, WebSocket, and account endpoints.</li>
        <li><Link href="/docs/tutorials/deploy-contract">Deploy a Contract</Link> — get a contract address to read from.</li>
      </ul>
    </DocPage>
  );
}
