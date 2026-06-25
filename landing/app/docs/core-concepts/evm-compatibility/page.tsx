"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function EvmCompatibilityPage() {
  return (
    <DocPage title="EVM Compatibility">
      <p>
        Fluidic embeds <strong>revm</strong> to execute raw Ethereum transactions. The EVM state — account balances, nonces, contract bytecode, and contract storage — is persisted across synthesis ticks. You can deploy contracts, read state with <code>eth_call</code>, and verify code with <code>eth_getCode</code> exactly like on any EVM chain.
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
        <li><code>eth_chainId</code> / <code>net_version</code></li>
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

      <h2>Funding EVM wallets</h2>
      <p>
        EVM balances are backed by Fluidic accounts. Before you can deploy or call a contract, fund the EVM address&apos;s mapped Fluidic account:
      </p>
      <pre><code>{`curl -X POST https://api.testnet.fluidic.foundation/api/evm/faucet \\
  -H "Content-Type: application/json" \\
  -d '{"address":"0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1"}'`}</code></pre>

      <h2>Deploy a contract with Foundry</h2>
      <pre><code>{`# foundry.toml
[rpc_endpoints]
fluidic = "https://api.testnet.fluidic.foundation/rpc"

# Deploy (sign with a funded EVM private key)
forge create src/MyContract.sol:MyContract \\
  --rpc-url fluidic \\
  --private-key $FLUIDIC_PK \\
  --chain 990492`}</code></pre>

      <h2>Deploy a contract with Hardhat</h2>
      <pre><code>{`// hardhat.config.ts
const config: HardhatUserConfig = {
  networks: {
    fluidic: {
      url: "https://api.testnet.fluidic.foundation/rpc",
      chainId: 990492,
      accounts: [process.env.FLUIDIC_PK!],
    },
  },
};

// scripts/deploy.ts
const factory = await ethers.getContractFactory("MyContract");
const contract = await factory.deploy();
await contract.waitForDeployment();
console.log("deployed to:", await contract.getAddress());`}</code></pre>

      <h2>Read and write from the SDK</h2>
      <pre><code>{`import { createClient } from "@fluidic/sdk";

const client = createClient({ apiUrl: "https://api.testnet.fluidic.foundation" });

// Submit any signed EIP-155 transaction
const txHash = await client.submitEvm({
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
});

// Read state via the standard RPC
const result = await fetch("https://api.testnet.fluidic.foundation/rpc", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: "eth_call",
    params: [{ to: contractAddress, data: calldata }, "latest"],
  }),
});
const { result: returnData } = await result.json();`}</code></pre>

      <h2>How balances work for EVM wallets</h2>
      <p>
        EVM balances are backed by the Fluidic wave-field. Before an EVM transaction can execute, the sender must have a Fluidic balance at the deterministic account derived from their EVM address. Use the <code>/api/account/register</code> and <code>/faucet</code> endpoints, or send a stateful transfer to that account, to fund a wallet.
      </p>

      <h2>Persistence guarantees</h2>
      <p>
        EVM state is committed at the end of every synthesis tick and written to the node&apos;s on-disk snapshot. Restarting a node replays the snapshot, so deployed contracts, nonces, and storage survive reboots. The snapshot also propagates through consensus: each synthesis certificate includes an <code>evm_root</code>, making EVM state auditable and reproducible across the mesh.
      </p>

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/tutorials/deploy-contract">Deploy a Contract tutorial</Link> — step-by-step Foundry deployment.</li>
        <li><Link href="/docs/api-reference/evm-rpc">EVM RPC reference</Link> — method details.</li>
        <li><Link href="/docs/core-concepts/accounts">Accounts</Link> — EVM-to-Fluidic account mapping.</li>
      </ul>
    </DocPage>
  );
}
