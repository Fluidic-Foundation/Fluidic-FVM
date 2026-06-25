"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function EvmRpcPage() {
  return (
    <DocPage title="EVM RPC">
      <p>
        Fluidic exposes a standard Ethereum JSON-RPC endpoint at <code>/rpc</code>. You can use it with Foundry, Hardhat, ethers.js, viem, or any EVM tooling.
      </p>

      <h2>Network parameters</h2>
      <table>
        <thead>
          <tr><th>Parameter</th><th>Value</th></tr>
        </thead>
        <tbody>
          <tr><td>RPC URL</td><td><code>https://api.testnet.fluidic.foundation/rpc</code></td></tr>
          <tr><td>Chain ID</td><td><code>0xf1d1c</code> (decimal <code>990492</code>)</td></tr>
          <tr><td>Currency</td><td>WAVE</td></tr>
          <tr><td>Gas price</td><td>Ignored; execution is metered but currently subsidized</td></tr>
          <tr><td>Block number</td><td>Maps to the latest synthesis tick</td></tr>
        </tbody>
      </table>

      <h2>Supported JSON-RPC methods</h2>
      <ul>
        <li><code>eth_chainId</code></li>
        <li><code>net_version</code></li>
        <li><code>eth_gasPrice</code></li>
        <li><code>eth_blockNumber</code></li>
        <li><code>eth_getBlockByNumber</code></li>
        <li><code>eth_getBalance</code></li>
        <li><code>eth_getTransactionCount</code></li>
        <li><code>eth_getCode</code></li>
        <li><code>eth_call</code></li>
        <li><code>eth_estimateGas</code></li>
        <li><code>eth_sendRawTransaction</code></li>
        <li><code>eth_getTransactionReceipt</code></li>
        <li><code>web3_clientVersion</code></li>
      </ul>

      <h2>Fund an EVM address</h2>
      <p>
        Before sending transactions, fund the address through the EVM faucet:
      </p>
      <pre><code>{`curl -X POST https://api.testnet.fluidic.foundation/api/evm/faucet \\
  -H "Content-Type: application/json" \\
  -d '{"address":"0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1"}'`}</code></pre>

      <h2>Example: read contract state</h2>
      <pre><code>{`curl -X POST https://api.testnet.fluidic.foundation/rpc \\
  -H "Content-Type: application/json" \\
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "eth_call",
    "params": [
      { "to": "0xC89Ce4735882C9F0f0FE26686c53074E09B0D550", "data": "0x06661abd" },
      "latest"
    ]
  }'`}</code></pre>

      <h2>Example: send a raw transaction</h2>
      <pre><code>{`curl -X POST https://api.testnet.fluidic.foundation/rpc \\
  -H "Content-Type: application/json" \\
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "eth_sendRawTransaction",
    "params": ["0x..."]
  }'`}</code></pre>

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/core-concepts/evm-compatibility">EVM Compatibility</Link> — how revm is integrated.</li>
        <li><Link href="/docs/tutorials/deploy-contract">Deploy a Contract</Link> — Foundry walkthrough.</li>
        <li><Link href="/docs/getting-started/testnet">Testnet</Link> — public RPC and faucet.</li>
      </ul>
    </DocPage>
  );
}
