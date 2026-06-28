"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function QuickstartPage() {
  return (
    <DocPage title="Quickstart">
      <p>
        This guide gets you from zero to a live Fluidic node and your first submitted shift. You only need Docker and Node.js installed.
      </p>

      <h2>1. Run a local node</h2>
      <pre><code>{`docker run -d --name fluidic-node \\
  --restart unless-stopped \\
  -p 8080:8080 -p 7000:7000 \\
  -e OSCILLATOR_ID=12345 \\
  -e PEERS="34.56.159.76:7000" \\
  -e FLUIDIC_DATA_DIR=/data \\
  -v "$HOME/fluidic-data:/data" \\
  us-central1-docker.pkg.dev/project-934c3e12-e0e7-4811-810/fluidic/mesh-node:latest`}</code></pre>
      <p>
        Use a <strong>unique numeric</strong> <code>OSCILLATOR_ID</code> (e.g. <code>12345</code>, not the default <code>node-1</code>). Two nodes with the same ID share a keypair and will slash each other. Mount <code>/data</code> so your snapshot and identity survive restarts.
      </p>
      <p>
        The container exposes the HTTP API on port <code>8080</code> and gossip on port <code>7000</code>. On first boot the node derives an operator keypair, seeds a genesis balance, and stakes it so it can produce certificates immediately.
      </p>

      <h2>2. Install the SDK</h2>
      <pre><code>{`npm install @fluidic/sdk
# or link from the repo
npm link /path/to/fluidic/sdk/typescript`}</code></pre>

      <h2>3. Submit a swap</h2>
      <pre><code>{`import { FluidicClient, FluidicKeypair, submitSwap } from "@fluidic/sdk";

const client = new FluidicClient({ apiUrl: "http://localhost:8080" });
const wallet = FluidicKeypair.generate();
await client.register(wallet.publicKeyHex);

const { poolInHash } = await submitSwap(client, {
  signer: wallet,
  direction: "WAVE_TO_USDC",
  amount: 1_000_000n,
});

console.log("swap submitted:", poolInHash);`}</code></pre>

      <h2>Next steps</h2>
      <ul>
        <li><Link href="/docs/getting-started/testnet">Connect to the public testnet</Link> instead of running locally.</li>
        <li><Link href="/docs/core-concepts/accounts">Read about accounts and keys</Link>.</li>
        <li><Link href="/docs/tutorials/build-dapp">Build a frontend</Link> that talks to the node.</li>
      </ul>
    </DocPage>
  );
}
