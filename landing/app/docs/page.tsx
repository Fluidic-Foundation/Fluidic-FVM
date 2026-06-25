"use client";

import { motion } from "framer-motion";
import {
  ArrowRight,
  BookOpen,
  Blocks,
  Box,
  Cpu,
  Globe,
  Layers,
  Radio,
  Shield,
  Terminal,
  Wallet,
  Zap,
} from "lucide-react";
import Link from "next/link";

const sections = [
  { id: "what-is-fluidic", title: "What is Fluidic?", icon: BookOpen },
  { id: "architecture", title: "Architecture", icon: Layers },
  { id: "accounts", title: "Accounts & keys", icon: Wallet },
  { id: "quickstart", title: "Quickstart", icon: Terminal },
  { id: "run-a-node", title: "Run a node", icon: Cpu },
  { id: "sdk", title: "SDK reference", icon: Box },
  { id: "api", title: "API reference", icon: Radio },
  { id: "web3-devs", title: "For Web3 devs", icon: Globe },
  { id: "evm", title: "EVM compatibility", icon: Blocks },
  { id: "deploy-tutorial", title: "Tutorial: deploy a contract", icon: Box },
  { id: "synthesis", title: "Synthesis & certificates", icon: Zap },
  { id: "validators", title: "Validators & staking", icon: Shield },
  { id: "quorum", title: "Quorum & finality", icon: Globe },
  { id: "tokenomics", title: "Tokenomics", icon: Cpu },
  { id: "testnet", title: "Testnet", icon: Globe },
  { id: "security", title: "Security", icon: Shield },
  { id: "whitepaper", title: "Whitepaper", icon: BookOpen },
  { id: "faq", title: "FAQ", icon: BookOpen },
];

export default function DocsPage() {
  return (
    <div className="relative min-h-screen bg-[#0D0D1F] text-[#F0F0F0]">
      <div className="grain" aria-hidden="true" />
      <div className="mesh-bg fixed inset-0 -z-10 opacity-40" />

      <nav className="sticky top-0 z-50 border-b border-white/5 bg-[#0D0D1F]/90 backdrop-blur-xl">
        <div className="mx-auto flex h-16 max-w-[1600px] items-center justify-between px-6">
          <Link href="/" className="group flex items-center gap-3 font-mono text-[12px] uppercase tracking-[0.2em] text-[#8A8AA3] transition-colors hover:text-[#00E6A7]">
            <img src="/fluidic-logo-new.png" alt="Fluidic" className="h-8 w-8 object-contain transition-transform duration-500 group-hover:scale-110" />
            <span>Fluidic</span>
          </Link>
          <div className="flex items-center gap-8 font-mono text-[11px] uppercase tracking-[0.2em] text-[#8A8AA3]">
            <Link href="/explorer/" className="transition-colors hover:text-[#00E6A7]">Explorer</Link>
            <Link href="/docs/" className="text-[#00E6A7]">Docs</Link>
            <a href="https://github.com/Fluidic-Foundation" target="_blank" rel="noreferrer" className="transition-colors hover:text-[#00E6A7]">GitHub</a>
          </div>
        </div>
      </nav>

      <div className="mx-auto flex max-w-[1600px] flex-col gap-12 px-6 py-16 lg:flex-row">
        <aside className="flex-shrink-0 lg:sticky lg:top-24 lg:h-fit lg:w-64">
          <div className="space-y-1">
            {sections.map((s) => (
              <a
                key={s.id}
                href={`#${s.id}`}
                className="group flex items-center gap-3 border-l border-white/10 py-3 pl-4 font-mono text-[11px] uppercase tracking-[0.15em] text-[#8A8AA3] transition-all hover:border-[#00E6A7] hover:bg-white/[0.02] hover:text-[#00E6A7]"
              >
                <span className="text-[#7700FF] group-hover:text-[#00E6A7]"><s.icon className="h-4 w-4" /></span>
                {s.title}
              </a>
            ))}
          </div>
        </aside>

        <motion.article
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6 }}
          className="prose-docs max-w-3xl"
        >
          <header className="mb-16 border-b border-white/10 pb-10">
            <h1 className="font-serif text-4xl font-light leading-[1.05] text-[#F0F0F0] md:text-6xl">
              Fluidic Documentation
            </h1>
            <p className="mt-4 font-mono text-[12px] leading-relaxed text-[#8A8AA3]">
              The continuous-wave state engine: permissionless nodes, NTT-aggregated commutative shifts,
              vector-clock DAG ordering, and BFT synthesis certificates.
            </p>
          </header>

          <Section id="what-is-fluidic" title="What is Fluidic?">
            <p>
              Fluidic is a <strong>blockless state-synthesis network</strong>. Instead of collecting transactions into blocks and ordering them through a leader, Fluidic nodes continuously ingest signed <strong>shifts</strong>, order stateful shifts in a vector-clock DAG, batch commutative shifts through Number-Theoretic Transforms, and synthesize the resulting state in periodic ticks.
            </p>
            <p>
              Every tick produces a <strong>Synthesis Certificate</strong>: a signed bundle containing Merkle roots of the commutative state, stateful DAG, balances, stake table, reward pool, and EVM transactions. Operators sign certificates, and once a quorum of stake-weighted signatures is observed, the tick is considered finalized.
            </p>
            <h3>Core ideas</h3>
            <ul>
              <li><strong>No blocks.</strong> State advances continuously through synthesis ticks.</li>
              <li><strong>No mempool auction.</strong> Shifts are gossiped and causally ordered, not front-run.</li>
              <li><strong>Parallel by default.</strong> Commutative shifts merge in NTT windows; stateful shifts merge through the DAG.</li>
              <li><strong>Permissionless.</strong> Anyone can run a synthesis node and earn rewards.</li>
              <li><strong>EVM-compatible.</strong> Raw Ethereum transactions execute inside a revm sandbox.</li>
            </ul>
          </Section>

          <Section id="architecture" title="Architecture">
            <h3>Commutative vs. stateful shifts</h3>
            <table>
              <thead>
                <tr><th>Property</th><th>Commutative shift</th><th>Stateful shift</th></tr>
              </thead>
              <tbody>
                <tr><td>Ordering</td><td>None required</td><td>Causal (vector-clock DAG)</td></tr>
                <tr><td>Examples</td><td>AMM swaps, streaming payments</td><td>Account transfers, EVM txs</td></tr>
                <tr><td>Aggregation</td><td>NTT batch window</td><td>Topological DAG order</td></tr>
                <tr><td>Conflict handling</td><td>Natural addition</td><td>Double-spend rejection</td></tr>
              </tbody>
            </table>

            <h3>The oscillator</h3>
            <p>
              Each node runs an <strong>Oscillator</strong>. Every <code>SYNTHESIS_INTERVAL_MS</code> it:
            </p>
            <ol>
              <li>Applies metabolic burn and distributes rewards.</li>
              <li>Moves pending stateful shifts into the DAG and detects double-spends.</li>
              <li>Promotes accepted shifts to finalized after a confirmation depth.</li>
              <li>Batches commutative deltas through an NTT window.</li>
              <li>Applies stateful and EVM transactions in topological/nonce order.</li>
              <li>Computes Merkle roots and, if staked, signs a Synthesis Certificate.</li>
            </ol>

            <h3>Concurrency domains</h3>
            <p>
              A <strong>domain</strong> is a 32-byte scope tag (e.g., <code>DEFAULT_DEX_DOMAIN</code>). Commutative shifts within the same domain are batched together; stateful shifts carry a domain and are validated against that domain’s DAG. Domains allow many independent applications to share the mesh without contending for global lock.
            </p>

            <h3>Networking</h3>
            <p>
              Nodes connect over TCP gossip. Each node binds a gossip socket (default <code>0.0.0.0:7000</code>) and dials the comma-separated <code>PEERS</code>. Signed shifts, registrations, stakes, and certificates are propagated through the mesh.
            </p>
          </Section>

          <Section id="accounts" title="Accounts & keys">
            <p>
              Fluidic uses <strong>Ed25519</strong> keypairs. A public key is hashed to a 32-byte <strong>AccountId</strong>. From that account, the node derives token accounts (WAVE, USDC) using deterministic hashes of the account and asset tag.
            </p>
            <h3>Creating a wallet with the SDK</h3>
            <pre><code>{`import { FluidicKeypair } from "@fluidic/sdk";

const wallet = FluidicKeypair.generate();
console.log("account:", wallet.accountId);
console.log("public key:", wallet.publicKeyHex);`}</code></pre>
            <h3>Registering with a node</h3>
            <p>
              Before a node accepts stateful shifts from an account, the public key must be registered in its key registry. The <code>/api/account/register</code> endpoint records the key and seeds token accounts from a faucet.
            </p>
            <pre><code>{`const client = new FluidicClient({ apiUrl: "http://localhost:8080" });
await client.register(wallet.publicKeyHex);`}</code></pre>
          </Section>

          <Section id="quickstart" title="Quickstart">
            <h3>1. Run a local node</h3>
            <pre><code>{`docker run -d --name fluidic-node \\
  -p 8080:8080 -p 7000:7000 \\
  -e OSCILLATOR_ID=node-1 \\
  -e PEERS="34.56.159.76:7000" \\
  ghcr.io/Fluidic-Foundation/Fluidic-FVM:latest`}</code></pre>
            <h3>2. Install the SDK</h3>
            <pre><code>{`npm install @fluidic/sdk
# or link from the repo
npm link /path/to/fluidic/sdk/typescript`}</code></pre>
            <h3>3. Submit a swap</h3>
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
          </Section>

          <Section id="run-a-node" title="Run a node">
            <p>
              The node binary is configured through environment variables:
            </p>
            <table>
              <thead><tr><th>Variable</th><th>Default</th><th>Description</th></tr></thead>
              <tbody>
                <tr><td>OSCILLATOR_ID</td><td>0</td><td>Node identity; must be a number or end with one</td></tr>
                <tr><td>API_PORT</td><td>8080</td><td>HTTP/WebSocket API port</td></tr>
                <tr><td>BIND_ADDR</td><td>0.0.0.0:7000</td><td>TCP gossip bind address</td></tr>
                <tr><td>PEERS</td><td>"34.56.159.76:7000"</td><td>Testnet gossip seed (comma-separated list supported)</td></tr>
                <tr><td>SYNTHESIS_INTERVAL_MS</td><td>1000</td><td>Tick interval</td></tr>
              </tbody>
            </table>
            <h3>Join the testnet</h3>
            <pre><code>{`docker run -d --name fluidic-node \\
  -p 8080:8080 -p 7000:7000 \\
  -e OSCILLATOR_ID=node-1 \\
  -e PEERS="34.56.159.76:7000" \\
  ghcr.io/Fluidic-Foundation/Fluidic-FVM:latest`}</code></pre>
            <p>
              On first boot the node derives a deterministic operator keypair, seeds a genesis balance, and stakes it so it can produce certificates immediately.
            </p>
          </Section>

          <Section id="sdk" title="SDK reference">
            <h3>FluidicClient</h3>
            <pre><code>{`const client = new FluidicClient({
  apiUrl: "http://localhost:8080",
});`}</code></pre>
            <h3>Key methods</h3>
            <ul>
              <li><code>register(publicKeyHex)</code> — register an account and faucet seed.</li>
              <li><code>submitStateful(shift)</code> — submit a signed stateful shift.</li>
              <li><code>submitCommutative(shift)</code> — submit a commutative delta.</li>
              <li><code>submitEvm(tx)</code> — submit a raw EVM transaction.</li>
              <li><code>getState()</code> — fetch live pool state.</li>
              <li><code>waitForStatus(hash, status, opts)</code> — poll finality.</li>
            </ul>
            <h3>Building shifts</h3>
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
          </Section>

          <Section id="api" title="API reference">
            <h3>State</h3>
            <ul>
              <li><code>GET /api/state</code> — pool reserves, price, throughput, applied counts.</li>
              <li><code>GET /api/ws</code> — WebSocket stream of state snapshots.</li>
            </ul>
            <h3>Accounts</h3>
            <ul>
              <li><code>POST /api/account/register</code> — register a public key, returns derived token accounts.</li>
              <li><code>GET /api/account/:id/balance</code> — WAVE/USDC balances.</li>
              <li><code>GET /api/operators</code> — list staked operators.</li>
            </ul>
            <h3>Shifts</h3>
            <ul>
              <li><code>POST /api/shift/stateful</code> — submit a stateful shift.</li>
              <li><code>POST /api/shift/commutative</code> — submit a commutative shift.</li>
              <li><code>GET /api/shift/:hash/status</code> — <code>unknown | accepted | finalized | rejected</code>.</li>
              <li><code>GET /api/shifts/recent?limit=N</code> — recent accepted shifts.</li>
            </ul>
            <h3>EVM</h3>
            <ul>
              <li><code>POST /api/evm/tx</code> — submit a signed raw Ethereum transaction.</li>
              <li><code>POST /rpc</code> — Ethereum JSON-RPC namespace (chain, balances, sendRaw, call, code, receipts).</li>
            </ul>
            <h3>Consensus</h3>
            <ul>
              <li><code>GET /api/certificate/:tick</code> — certificate for a tick.</li>
              <li><code>GET /api/quorum/:tick</code> — quorum status and signatures.</li>
              <li><code>GET /api/ticks/recent?limit=N</code> — recent synthesis ticks.</li>
              <li><code>GET /api/ticks/:tick</code> — single tick summary.</li>
            </ul>
            <h3>Operator</h3>
            <ul>
              <li><code>GET /api/operator/info</code> — local operator account/stake.</li>
              <li><code>POST /api/operator/stake</code> — stake additional WAVE.</li>
            </ul>
          </Section>

          <Section id="web3-devs" title="For Web3 developers">
            <p>
              If you are coming from Ethereum or Solana, the fastest way to reason about Fluidic is to map concepts you already know. Fluidic is <strong>not</strong> an EVM chain and <strong>not</strong> an SVM chain, but it borrows the best parts of both: deterministic contract execution from Ethereum, and parallel, account-centric state from Solana.
            </p>

            <h3>Concept map</h3>
            <table>
              <thead>
                <tr><th>Ethereum / EVM</th><th>Solana / SVM</th><th>Fluidic</th></tr>
              </thead>
              <tbody>
                <tr><td>Block</td><td>Slot</td><td><strong>Synthesis tick</strong></td></tr>
                <tr><td>Transaction</td><td>Transaction / Instruction</td><td><strong>Shift</strong> (stateful or commutative)</td></tr>
                <tr><td>Smart contract</td><td>Program</td><td><strong>EVM contract</strong> or <strong>domain</strong></td></tr>
                <tr><td>Account (nonce + balance)</td><td>Account</td><td><strong>AccountId</strong> (Ed25519-derived)</td></tr>
                <tr><td>Contract storage</td><td>Program-derived address (PDA)</td><td><strong>EVM account storage</strong> in revm, persisted across ticks</td></tr>
                <tr><td>Mempool / block builder</td><td>Leader scheduling</td><td><strong>Gossip + oscillator</strong> (no leader)</td></tr>
                <tr><td>Gas fee</td><td>Compute units + priority fee</td><td><strong>Metabolic burn</strong> per tick (currently subsidized)</td></tr>
                <tr><td>Finality</td><td>Root + tower BFT</td><td><strong>2/3+1 stake-weighted certificates</strong></td></tr>
              </tbody>
            </table>

            <h3>Stateful vs. commutative shifts</h3>
            <p>
              On Ethereum every transaction is linearly ordered. On Solana, instructions within a transaction can execute in parallel if they touch disjoint accounts. Fluidic makes that distinction explicit at the protocol level:
            </p>
            <ul>
              <li><strong>Stateful shifts</strong> need causal ordering (transfers, EVM transactions). They go into the vector-clock DAG, exactly like Ethereum transactions need a total order.</li>
              <li><strong>Commutative shifts</strong> are order-independent (AMM pool deltas, streaming payments). They are batched with an NTT, similar to how Solana parallelizes non-conflicting instructions.</li>
            </ul>

            <h3>Accounts and programs</h3>
            <p>
              A Fluidic account is a 32-byte <code>AccountId</code> derived from an Ed25519 public key. The node automatically derives WAVE and USDC token accounts from it. For EVM compatibility, any 20-byte Ethereum address is deterministically mapped to a 32-byte Fluidic account using <code>blake3("fluidic:evm-account:v1" || address)</code>, so an EVM wallet's balance is readable through both RPCs.
            </p>
            <p>
              EVM contracts are just EVM accounts with code. They execute inside <strong>revm</strong>, the same interpreter used by Optimism, Arbitrum, and many L2s. Contract bytecode and storage persist across synthesis ticks, so you can deploy once and interact forever — exactly like on Ethereum.
            </p>

            <h3>From Base Sepolia to Fluidic</h3>
            <p>
              The recommended migration path for Base Sepolia contracts is to redeploy on Fluidic's EVM layer. Fluidic uses its own chain ID (<code>0xf1d1c</code> / <code>990492</code>), its own RPC URL, and deterministic balances derived from the Fluidic wave-field. You keep the same Solidity code, the same contract addresses (after deployment), and the same tooling.
            </p>
          </Section>

          <Section id="evm" title="EVM compatibility">
            <p>
              Fluidic embeds <strong>revm</strong> to execute raw Ethereum transactions. The EVM state — account balances, nonces, contract bytecode, and contract storage — is persisted across synthesis ticks. You can deploy contracts, read state with <code>eth_call</code>, and verify code with <code>eth_getCode</code> exactly like on any EVM chain.
            </p>

            <h3>Network parameters</h3>
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

            <h3>Supported JSON-RPC methods</h3>
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
            <h3>Funding EVM wallets</h3>
            <p>
              EVM balances are backed by Fluidic accounts. Before you can deploy or call a contract, fund the EVM address's mapped Fluidic account:
            </p>
            <pre><code>{`curl -X POST https://api.testnet.fluidic.foundation/api/evm/faucet \\
  -H "Content-Type: application/json" \\
  -d '{"address":"0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1"}'`}</code></pre>

            <h3>Deploy a contract with Foundry</h3>
            <pre><code>{`# foundry.toml
[rpc_endpoints]
fluidic = "https://api.testnet.fluidic.foundation/rpc"

# Deploy (sign with a funded EVM private key)
forge create src/MyContract.sol:MyContract \\
  --rpc-url fluidic \\
  --private-key $FLUIDIC_PK \\
  --chain 990492`}</code></pre>

            <h3>Deploy a contract with Hardhat</h3>
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

            <h3>Read and write from the SDK</h3>
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

            <h3>How balances work for EVM wallets</h3>
            <p>
              EVM balances are backed by the Fluidic wave-field. Before an EVM transaction can execute, the sender must have a Fluidic balance at the deterministic account derived from their EVM address. Use the <code>/api/account/register</code> and <code>/faucet</code> endpoints, or send a stateful transfer to that account, to fund a wallet.
            </p>

            <h3>Persistence guarantees</h3>
            <p>
              EVM state is committed at the end of every synthesis tick and written to the node's on-disk snapshot. Restarting a node replays the snapshot, so deployed contracts, nonces, and storage survive reboots. The snapshot also propagates through consensus: each synthesis certificate includes an <code>evm_root</code>, making EVM state auditable and reproducible across the mesh.
            </p>
          </Section>

          <Section id="deploy-tutorial" title="Tutorial: deploy a contract">
            <p>
              This tutorial deploys a simple <code>Counter</code> Solidity contract on the Fluidic testnet. The steps are the same for any Base Sepolia contract: fund the EVM wallet, compile with your normal toolchain, and deploy to the Fluidic RPC.
            </p>

            <h3>Prerequisites</h3>
            <ul>
              <li><a href="https://book.getfoundry.sh" target="_blank" rel="noreferrer">Foundry</a> installed (<code>forge</code>, <code>cast</code>).</li>
              <li>An EVM private key. This tutorial uses the standard Ganache test key <code>0x4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d</code> (address <code>0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1</code>). Do not use this key for real funds.</li>
            </ul>

            <h3>1. Create the project</h3>
            <pre><code>{`mkdir counter && cd counter

cat > foundry.toml <<'EOF'
[profile.default]
src = "src"
out = "out"
libs = ["lib"]

[rpc_endpoints]
fluidic = "https://api.testnet.fluidic.foundation/rpc"
EOF

cat > src/Counter.sol <<'EOF'
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

contract Counter {
    uint256 public count;
    event Incremented(uint256 newCount);

    function increment() external {
        count += 1;
        emit Incremented(count);
    }

    function set(uint256 x) external {
        count = x;
    }
}
EOF

forge build`}</code></pre>

            <h3>2. Fund the deployer</h3>
            <pre><code>{`export PK=0x4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d
export ADDR=$(cast wallet address --private-key $PK)

curl -X POST https://api.testnet.fluidic.foundation/api/evm/faucet \\
  -H "Content-Type: application/json" \\
  -d "{\"address\":\"$ADDR\"}"

# Check the mapped Fluidic balance
cast balance --rpc-url https://api.testnet.fluidic.foundation/rpc $ADDR`}</code></pre>

            <h3>3. Deploy</h3>
            <pre><code>{`forge create src/Counter.sol:Counter \\
  --rpc-url https://api.testnet.fluidic.foundation/rpc \\
  --private-key $PK \\
  --chain 990492 \\
  --legacy \\
  --broadcast`}</code></pre>

            <h3>4. Verify deployment</h3>
            <p>
              If the transaction succeeds, Forge prints the deployed address. You can read it with <code>eth_getCode</code>:
            </p>
            <pre><code>{`cast code --rpc-url https://api.testnet.fluidic.foundation/rpc 0xC89Ce4735882C9F0f0FE26686c53074E09B0D550`}</code></pre>

            <h3>5. Read and write state</h3>
            <pre><code>{`# Read count (should be 0)
cast call 0xC89Ce4735882C9F0f0FE26686c53074E09B0D550 \\
  "count()(uint256)" \\
  --rpc-url https://api.testnet.fluidic.foundation/rpc

# Increment
cast send 0xC89Ce4735882C9F0f0FE26686c53074E09B0D550 \\
  "increment()" \\
  --rpc-url https://api.testnet.fluidic.foundation/rpc \\
  --private-key $PK \\
  --chain 990492 \\
  --legacy

# Read count again (should be 1)
cast call 0xC89Ce4735882C9F0f0FE26686c53074E09B0D550 \\
  "count()(uint256)" \\
  --rpc-url https://api.testnet.fluidic.foundation/rpc`}</code></pre>

            <h3>6. Bring a Base Sepolia contract across</h3>
            <p>
              To migrate an existing Base Sepolia contract, copy its source into the same Foundry project, keep the same compiler version, and run the same <code>forge create</code> command. The contract will receive a deterministic address on Fluidic based on the deployer nonce. If you need the exact same address as on Base Sepolia, use the same deployer account and nonce; otherwise treat Fluidic as a fresh deployment and update your frontend's contract addresses.
            </p>
            <p>
              For contracts that read Base Sepolia state (e.g., an oracle or bridge), option 1 from the introduction is still the right path: lock assets on Base Sepolia and run a relayer that pushes state into Fluidic.
            </p>
          </Section>

          <Section id="synthesis" title="Synthesis & certificates">
            <p>
              A <strong>synthesis tick</strong> is a periodic state transition. At each tick the oscillator applies metabolic burn, drains pending queues into the DAG, finalizes previous shifts, applies commutative batches, executes EVM transactions, and computes Merkle roots.
            </p>
            <h3>Synthesis certificate fields</h3>
            <ul>
              <li><code>tick</code> — monotonic tick number.</li>
              <li><code>operator</code> — signing operator account.</li>
              <li><code>commutative_root</code> — Merkle root of commutative deltas.</li>
              <li><code>stateful_root</code> — Merkle root of applied stateful hashes.</li>
              <li><code>balances_root</code> — root of account balances.</li>
              <li><code>stake_root</code> — root of the stake table.</li>
              <li><code>reward_root</code> — root of the reward pool.</li>
              <li><code>evm_root</code> — root of EVM transactions.</li>
            </ul>
            <p>
              Certificates are gossiped so peers can form quorums. Each certificate is deterministically hashable, making state reproducible from the same inputs.
            </p>
          </Section>

          <Section id="validators" title="Validators & staking">
            <p>
              A node becomes a validator when its operator account is <strong>staked</strong>. On first boot the node seeds a genesis balance and stakes it automatically. Additional stake can be added through <code>/api/operator/stake</code>.
            </p>
            <h3>Rewards</h3>
            <p>
              Every synthesis tick applies metabolic burn. The burned amount is distributed to staked operators proportional to their stake. Operator rewards accrue in the reward pool and can be claimed via the stake table.
            </p>
            <h3>Minimum stake</h3>
            <p>
              The default minimum stake is <code>1e18</code> units. A node whose operator stake is below this threshold can still run and ingest shifts, but it will not sign synthesis certificates.
            </p>
          </Section>

          <Section id="quorum" title="Quorum & finality">
            <p>
              Finality is BFT. A tick is finalized when the <code>CertificateTracker</code> observes signatures from operators holding at least <code>2/3 + 1</code> of total stake. Conflicting certificates for the same tick are detected and the offending operator is slashed.
            </p>
            <p>
              Stateful shifts reach <code>finalized</code> status after surviving <code>FINALIZATION_DEPTH</code> synthesis ticks without a conflicting double-spend being accepted into the DAG.
            </p>
          </Section>

          <Section id="tokenomics" title="Tokenomics">
            <p>
              WAVE is the native unit of account. It is used for staking, metabolic burn, and reward distribution.
            </p>
            <h3>Metabolic burn</h3>
            <p>
              Every synthesis tick burns a deterministic amount. Burn is computed with integer arithmetic from a per-second rate and elapsed nanoseconds, avoiding floating-point drift.
            </p>
            <h3>Issuance and rewards</h3>
            <p>
              Genesis balances seed operators and faucet accounts. New units enter circulation through faucet drips (testnet) and operator rewards (mainnet). The reward distribution is stake-weighted and occurs every tick.
            </p>
          </Section>

          <Section id="testnet" title="Testnet">
            <p>
              The public testnet is live and permissionless. Use these endpoints:
            </p>
            <table>
              <thead><tr><th>Resource</th><th>Address</th></tr></thead>
              <tbody>
                <tr><td>API</td><td><code>https://api.testnet.fluidic.foundation</code></td></tr>
                <tr><td>Explorer</td><td><code>https://testnet.fluidic.foundation/explorer.html</code></td></tr>
                <tr><td>Gossip seed</td><td><code>34.56.159.76:7000</code></td></tr>
                <tr><td>Faucet</td><td><code>POST /faucet</code> on the API domain</td></tr>
              </tbody>
            </table>
            <p>
              To join, run the node container with <code>PEERS=34.56.159.76:7000</code>.
            </p>
          </Section>

          <Section id="security" title="Security">
            <p>
              Fluidic’s security model rests on four pillars:
            </p>
            <ul>
              <li><strong>Cryptography.</strong> All shifts are signed with Ed25519. Certificates are signed by staked operators.</li>
              <li><strong>Causal ordering.</strong> Stateful shifts are ordered in a vector-clock DAG; conflicts are rejected.</li>
              <li><strong>BFT quorum.</strong> Finality requires a two-thirds-plus-one stake-weighted certificate quorum.</li>
              <li><strong>Determinism.</strong> Merkle roots and synthesis results are deterministic from the same inputs, enabling audit and replay.</li>
            </ul>
            <p>
              The codebase has been audited internally across consensus, cryptography, networking, economics, and EVM execution. Known gaps are tracked and patched in the main repo.
            </p>
          </Section>

          <Section id="whitepaper" title="Whitepaper">
            <p>
              The Fluidic whitepaper describes the protocol in depth: the move from sequential ledgers to continuous wave-fields, the NTT aggregation proof, the vector-clock DAG, the metabolic burn model, and the BFT consensus layer.
            </p>
            <Link href="/whitepaper.pdf" className="docs-btn">
              Read the whitepaper <ArrowRight className="ml-2 h-3 w-3" />
            </Link>
          </Section>

          <Section id="faq" title="FAQ">
            <h3>Do I need a validator node to use Fluidic?</h3>
            <p>
              No. Users and developers interact through the HTTP/WebSocket API. Running a node is only required if you want to validate the network and earn rewards.
            </p>
            <h3>How is this different from a blockchain?</h3>
            <p>
              Blockchains order transactions into blocks; Fluidic synthesizes continuous shifts. Commutative operations need no ordering, and stateful operations are causally ordered only where necessary.
            </p>
            <h3>What can I build?</h3>
            <p>
              Anything that benefits from high throughput, causal ordering, and sub-second finality: AMMs, orderbooks, streaming payments, agent coordination, on-chain games, and EVM-compatible dApps.
            </p>
            <h3>Is there a token?</h3>
            <p>
              WAVE is the native unit. On the testnet it is used for staking and rewards but has no real-world value. Mainnet tokenomics will be announced later.
            </p>
          </Section>

          <footer className="mt-20 border-t border-white/10 pt-10">
            <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-[#8A8AA3]">
              FLUIDIC FOUNDATION — CONTINUOUS-WAVE STATE SYNTHESIS
            </p>
          </footer>
        </motion.article>
      </div>

      <style jsx global>{`
        .prose-docs h2 {
          margin-top: 3rem;
          margin-bottom: 1.25rem;
          font-family: var(--font-fraunces), serif;
          font-size: 1.875rem;
          font-weight: 300;
          letter-spacing: -0.02em;
          color: #00e6a7;
        }
        .prose-docs h3 {
          margin-top: 2rem;
          margin-bottom: 0.75rem;
          font-family: var(--font-fraunces), serif;
          font-size: 1.25rem;
          font-weight: 400;
          color: #f0f0f0;
        }
        .prose-docs p {
          margin-bottom: 1rem;
          font-size: 0.9375rem;
          line-height: 1.75;
          color: #8a8aa3;
        }
        .prose-docs strong {
          color: #f0f0f0;
          font-weight: 500;
        }
        .prose-docs blockquote {
          margin: 1.5rem 0;
          border-left: 2px solid #7700ff;
          padding-left: 1.25rem;
          font-style: italic;
          color: #f0f0f0;
        }
        .prose-docs ul,
        .prose-docs ol {
          margin-bottom: 1.25rem;
          padding-left: 1.25rem;
          color: #8a8aa3;
        }
        .prose-docs li {
          margin-bottom: 0.5rem;
          font-size: 0.9375rem;
          line-height: 1.7;
        }
        .prose-docs code {
          font-family: var(--font-mono), monospace;
          font-size: 0.8125rem;
          background: rgba(240, 240, 240, 0.06);
          padding: 0.125rem 0.375rem;
          border-radius: 0.25rem;
          color: #00e6a7;
        }
        .prose-docs pre {
          margin: 1.25rem 0;
          overflow-x: auto;
          border: 1px solid rgba(240, 240, 240, 0.08);
          background: #0a0a14;
          padding: 1.25rem;
          border-radius: 0.25rem;
        }
        .prose-docs pre code {
          display: block;
          background: transparent;
          padding: 0;
          color: #f0f0f0;
          line-height: 1.65;
        }
        .prose-docs table {
          width: 100%;
          margin: 1.25rem 0;
          border-collapse: collapse;
          font-size: 0.875rem;
        }
        .prose-docs th,
        .prose-docs td {
          border: 1px solid rgba(240, 240, 240, 0.08);
          padding: 0.75rem 1rem;
          text-align: left;
        }
        .prose-docs th {
          background: rgba(119, 0, 255, 0.08);
          color: #f0f0f0;
          font-family: var(--font-mono), monospace;
          font-size: 0.75rem;
          text-transform: uppercase;
          letter-spacing: 0.1em;
        }
        .prose-docs td {
          color: #8a8aa3;
        }
      `}</style>
    </div>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
  return (
    <section id={id} className="scroll-mt-28">
      <div className="mb-6 flex items-center gap-3">
        <Radio className="h-4 w-4 text-[#7700FF]" />
        <h2>{title}</h2>
      </div>
      {children}
    </section>
  );
}
