"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function TestnetPage() {
  return (
    <DocPage title="Testnet">
      <p>
        The public Fluidic testnet is live and permissionless. You can interact with it through the HTTP API, the EVM RPC, the explorer, and the reference dApp.
      </p>

      <h2>Public endpoints</h2>
      <table>
        <thead>
          <tr><th>Resource</th><th>Address</th></tr>
        </thead>
        <tbody>
          <tr><td>API</td><td><code>https://api.testnet.fluidic.foundation</code></td></tr>
          <tr><td>EVM RPC</td><td><code>https://api.testnet.fluidic.foundation/rpc</code></td></tr>
          <tr><td>Explorer</td><td><code>https://testnet.fluidic.foundation/explorer/</code></td></tr>
          <tr><td>Reference dApp</td><td><code>https://app.testnet.fluidic.foundation</code></td></tr>
          <tr><td>Gossip seed</td><td><code>34.56.159.76:7000</code></td></tr>
          <tr><td>Container image</td><td><code>ghcr.io/Fluidic-Foundation/Fluidic-FVM:latest</code></td></tr>
        </tbody>
      </table>

      <h2>Join the testnet</h2>
      <pre><code>{`docker run -d --name fluidic-node \\
  -p 8080:8080 -p 7000:7000 \\
  -e OSCILLATOR_ID=node-1 \\
  -e PEERS="34.56.159.76:7000" \\
  ghcr.io/Fluidic-Foundation/Fluidic-FVM:latest`}</code></pre>

      <h2>Fund an EVM wallet</h2>
      <p>
        Use the EVM faucet to send WAVE to a 20-byte Ethereum address before deploying contracts or sending transactions.
      </p>
      <pre><code>{`curl -X POST https://api.testnet.fluidic.foundation/api/evm/faucet \\
  -H "Content-Type: application/json" \\
  -d '{"address":"0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1"}'`}</code></pre>

      <h2>Faucet for Fluidic accounts</h2>
      <p>
        Registering a new Ed25519 account through <code>/api/account/register</code> automatically seeds token accounts from the testnet faucet. For details see <Link href="/docs/core-concepts/accounts">Accounts</Link>.
      </p>

      <h2>Network parameters</h2>
      <table>
        <thead>
          <tr><th>Parameter</th><th>Value</th></tr>
        </thead>
        <tbody>
          <tr><td>Chain ID</td><td><code>990492</code> (hex <code>0xf1d1c</code>)</td></tr>
          <tr><td>Currency</td><td>WAVE</td></tr>
          <tr><td>Block number</td><td>Maps to the latest synthesis tick</td></tr>
          <tr><td>Gas price</td><td>Ignored; execution is metered but currently subsidized</td></tr>
        </tbody>
      </table>

      <h2>Next steps</h2>
      <ul>
        <li><Link href="/docs/tutorials/deploy-contract">Deploy a Solidity contract</Link> on testnet.</li>
        <li><Link href="/docs/api-reference/rest-api">Explore the REST API</Link>.</li>
        <li><Link href="/docs/api-reference/evm-rpc">Connect with standard EVM tooling</Link>.</li>
      </ul>
    </DocPage>
  );
}
