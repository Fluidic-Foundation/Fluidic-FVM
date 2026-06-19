"use client";

import { motion } from "framer-motion";
import { ArrowLeft, Terminal, BookOpen, Layers, Radio, Cpu, Globe } from "lucide-react";
import Link from "next/link";

const sections = [
  { id: "paradigm", title: "1. The Paradigm Shift", icon: <BookOpen className="h-4 w-4" /> },
  { id: "architecture", title: "2. Architecture Deep Dive", icon: <Layers className="h-4 w-4" /> },
  { id: "quickstart", title: "3. Quick Start", icon: <Terminal className="h-4 w-4" /> },
  { id: "sdk", title: "4. SDK Reference", icon: <Cpu className="h-4 w-4" /> },
  { id: "evm", title: "5. EVM Abstraction Layer", icon: <Globe className="h-4 w-4" /> },
];

export default function DocsPage() {
  return (
    <div className="relative min-h-screen bg-[#0D0D1F] text-[#F0F0F0]">
      <div className="grain" aria-hidden="true" />
      <div className="mesh-bg fixed inset-0 -z-10 opacity-40" />

      <nav className="sticky top-0 z-50 border-b border-white/5 bg-[#0D0D1F]/80 backdrop-blur-xl">
        <div className="mx-auto flex h-14 max-w-[1600px] items-center justify-between px-6">
          <Link href="/" className="group flex items-center gap-3 font-mono text-[11px] uppercase tracking-[0.2em] text-[#8A8AA3] transition-colors hover:text-[#00E6A7]">
            <div className="relative h-7 w-7 overflow-hidden">
              <img
                src="/fluidic-logo.png"
                alt="Fluidic"
                className="h-full w-full object-contain transition-transform duration-500 group-hover:scale-110"
              />
            </div>
            <span>Back to Fluidic</span>
          </Link>
          <span className="font-mono text-[10px] uppercase tracking-[0.3em] text-[#7700FF]">
            Developer Documentation
          </span>
        </div>
      </nav>

      <div className="mx-auto flex max-w-[1600px] flex-col gap-12 px-6 py-16 lg:flex-row">
        {/* Sidebar */}
        <aside className="lg:sticky lg:top-24 lg:h-fit lg:w-64">
          <div className="space-y-1">
            {sections.map((s) => (
              <a
                key={s.id}
                href={`#${s.id}`}
                className="group flex items-center gap-3 border-l border-white/10 py-3 pl-4 font-mono text-[11px] uppercase tracking-[0.15em] text-[#8A8AA3] transition-all hover:border-[#00E6A7] hover:bg-white/[0.02] hover:text-[#00E6A7]"
              >
                <span className="text-[#7700FF] group-hover:text-[#00E6A7]">{s.icon}</span>
                {s.title}
              </a>
            ))}
          </div>
        </aside>

        {/* Main content */}
        <motion.article
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6 }}
          className="prose-docs max-w-3xl"
        >
          <header className="mb-16 border-b border-white/10 pb-10">
            <h1 className="font-serif text-4xl font-light leading-[1.05] text-[#F0F0F0] md:text-6xl">
              Fluidic Developer Guide
            </h1>
            <p className="mt-4 font-mono text-[12px] leading-relaxed text-[#8A8AA3]">
              Parallel state synthesis for the post-blockchain internet. Install the mesh, inject your first signal, and deploy a horizontally-scaled decentralized service in under an hour.
            </p>
          </header>

          <Section id="paradigm" title="1. The Paradigm Shift">
            <p>
              Blockchains are sequential state machines. Every transaction must be ordered into a single, globally-agreed ledger. That design is elegant, but it is also the ceiling. When demand spikes, the entire network competes for the same block space. Gas prices explode. Reorgs happen. And every dApp developer learns to optimize around a global lock.
            </p>

            <p>
              <strong className="text-[#00E6A7]">Fluidic replaces the ledger with a mesh.</strong> Instead of ordering transactions into blocks, nodes synthesize overlapping <strong>Wave-Fields</strong> — causal, parallel representations of network state. A developer injects a <strong>Signal</strong>, the mesh gossips it through neighborhood concurrency domains, and independent operators synthesize the resulting state without ever needing a global chronological clock.
            </p>

            <h3>Old World (EVM)</h3>
            <ol className="list-decimal space-y-2 pl-5 font-mono text-[12px] text-[#8A8AA3]">
              <li>Sequential State Machine</li>
              <li>Transaction Mempool</li>
              <li>Block Construction</li>
              <li>Global Consensus Lock</li>
              <li>Ledger Update</li>
            </ol>

            <h3>New World (Fluidic)</h3>
            <ol className="list-decimal space-y-2 pl-5 font-mono text-[12px] text-[#8A8AA3]">
              <li>Asynchronous Wave-Field</li>
              <li>Signal Injection</li>
              <li>Neighborhood Gossip</li>
              <li>Localized Concurrency Domains</li>
              <li>Parallel Wave-Field Synthesis</li>
            </ol>

            <blockquote>
              In Fluidic, there is no block time. There is no mempool auction. There is no reorg. State updates are causally ordered inside your dApp’s domain and synthesized continuously by the mesh.
            </blockquote>

            <p>
              For you, the developer, this means you stop thinking about gas optimization, nonce management, and front-running. You think about signals: what state change do I want to propagate, and which concurrency domain owns it?
            </p>
          </Section>

          <Section id="architecture" title="2. Architecture Deep Dive">
            <h3>Signals vs. Transactions</h3>
            <p>
              A <strong>Transaction</strong> is a request to change a global ledger. It must be ordered relative to every other transaction. A <strong>Signal</strong> is an event vector: it carries intent, payload, a cryptographic proof, and a vector-clock entry. Signals do not wait in a global queue. They flow through the mesh and are merged where their causal histories overlap.
            </p>

            <table>
              <thead>
                <tr>
                  <th>Property</th>
                  <th>EVM Transaction</th>
                  <th>Fluidic Signal</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>Ordering</td>
                  <td>Global, sequential</td>
                  <td>Causal, domain-local</td>
                </tr>
                <tr>
                  <td>Execution</td>
                  <td>Block-by-block</td>
                  <td>Continuous synthesis</td>
                </tr>
                <tr>
                  <td>Failure mode</td>
                  <td>Reverts, gas loss</td>
                  <td>Rejected signal, no burn</td>
                </tr>
                <tr>
                  <td>Scaling</td>
                  <td>Vertical (bigger blocks)</td>
                  <td>Horizontal (more domains)</td>
                </tr>
              </tbody>
            </table>

            <h3>The Wave-Field Engine</h3>
            <p>
              Each Fluidic node maintains a <strong>Wave-Field</strong>: a locally-synthesized view of every concurrency domain it observes. The engine has three layers:
            </p>
            <ul>
              <li><strong>Signal Ingest</strong>: validate signatures, deduplicate, and route signals into the correct concurrency domain.</li>
              <li><strong>Vector-Clock DAG</strong>: order stateful signals causally. Conflicting signals are rejected; convergent signals are promoted to <code>Finalized</code> after a confirmation depth.</li>
              <li><strong>Wave-Field Synthesis</strong>: apply commutative updates through Number-Theoretic Transform (NTT) windows and apply stateful updates through the DAG. The result is a consistent local state snapshot.</li>
            </ul>

            <h3>Concurrency Domains</h3>
            <p>
              A <strong>Concurrency Domain</strong> is an isolated state partition: a dApp, a token pool, an operator registry, or any logical scope you define. Domains gossip independently. Adding nodes to a domain increases its synthesis throughput linearly, because work is partitioned by signal causality, not by a global lock.
            </p>

            <pre><code>{`// Declaring a domain is a single SDK call
const domain = await fluidic.domain({
  id: "dex-pool-alpha",
  consistency: "causal",   // "causal" | "commutative" | "strict"
  replicationFactor: 3,
});`}</code></pre>
          </Section>

          <Section id="quickstart" title="3. Quick Start: Zero to Live in 5 Minutes">
            <h3>Install the CLI</h3>
            <pre><code>{`npm install -g @fluidic/cli

# Verify
fluidic --version
# > fluidic/0.9.0`}</code></pre>

            <h3>Spin up a local dev node</h3>
            <pre><code>{`fluidic node --dev --domain local.test --api-port 8080

# Expected output:
# [INFO] mesh_node: synthesis: commutative=0 stateful=0 rejected=0
# [INFO] api: listening on 127.0.0.1:8080`}</code></pre>

            <h3>Your first signal</h3>
            <p>
              Create <code>hello-mesh.ts</code>. This script connects to your local node, defines a custom signal payload, injects it, and waits for synthesis confirmation.
            </p>
            <pre><code>{`import { FluidicClient } from "@fluidic/sdk";

async function main() {
  const client = await FluidicClient.connect({
    api: "http://localhost:8080",
    domain: "local.test",
  });

  // Register a keypair; the node will faucet token accounts.
  const account = await client.createAccount();
  console.log("account:", account.id);

  const signal = await client.signal({
    type: "chat.message",
    payload: {
      channel: "dev-relations",
      text: "Hello, Mesh.",
    },
    // SDK signs automatically with the registered keypair.
  });

  console.log("signal hash:", signal.hash);

  // Poll finality status.
  const status = await client.waitForStatus(signal.hash, "finalized", {
    timeoutMs: 5000,
  });

  console.log("status:", status.status); // "finalized"
}

main().catch(console.error);`}</code></pre>

            <p>Run it:</p>
            <pre><code>{`npx ts-node hello-mesh.ts`}</code></pre>
          </Section>

          <Section id="sdk" title="4. The Fluidic SDK Reference">
            <h3>Native Asset Creation</h3>
            <p>
              Fluidic does not use smart contracts for asset issuance. Assets are registered directly in the mesh-native <strong>Asset Registry</strong>. The registry stores metadata, supply policy, and minting rights as signed signals.
            </p>
            <pre><code>{`import { FluidicClient } from "@fluidic/sdk";

async function createMeshAsset() {
  const client = await FluidicClient.connect({
    api: "http://localhost:8080",
    domain: "assets.mainnet",
  });

  const asset = await client.assets.register({
    symbol: "WAVE",
    name: "Fluidic Wave",
    decimals: 12,
    totalSupply: 1_000_000_000_000_000_000_000n,
    mintingPolicy: {
      type: "fixed",
      authority: client.accountId,
    },
    metadata: {
      description: "Native mesh fuel",
      icon: "ipfs://Qm...",
    },
  });

  console.log("asset registered:", asset.id);

  // Mint an initial allocation to a pool domain.
  const mint = await client.assets.mint({
    assetId: asset.id,
    to: "pool-domain:wave-reserve",
    amount: 100_000_000_000_000n,
  });

  await client.waitForStatus(mint.hash, "finalized");
  console.log("mint finalized");
}`}</code></pre>

            <h3>Parallel Signal Handler</h3>
            <p>
              The following backend service layout shows how a dApp handles thousands of concurrent state-update signals. It uses domain isolation, batch ingestion, and explicit conflict resolution.
            </p>
            <pre><code>{`import { FluidicClient, SignalConflictError } from "@fluidic/sdk";
import { EventEmitter } from "events";

class OrderBookService extends EventEmitter {
  private client: FluidicClient;
  private domain: string;

  constructor(client: FluidicClient, domain: string) {
    super();
    this.client = client;
    this.domain = domain;
  }

  async start() {
    // Subscribe to all signals in this domain.
    this.client.onSignal(this.domain, async (signal) => {
      try {
        await this.handleSignal(signal);
      } catch (err) {
        if (err instanceof SignalConflictError) {
          // Causal conflict: queue for retry with updated predecessor set.
          await this.retryWithPredecessors(signal, err.predecessors);
        } else {
          this.emit("error", { signal, err });
        }
      }
    });
  }

  private async handleSignal(signal: any) {
    switch (signal.type) {
      case "order.place":
        return this.client.signal({
          type: "order.placed",
          payload: {
            orderId: signal.payload.orderId,
            side: signal.payload.side,
            price: signal.payload.price,
            amount: signal.payload.amount,
          },
          predecessors: [signal.hash],
          domain: this.domain,
        });

      case "order.cancel":
        return this.client.signal({
          type: "order.cancelled",
          payload: { orderId: signal.payload.orderId },
          predecessors: [signal.hash],
          domain: this.domain,
        });

      default:
        return;
    }
  }

  private async retryWithPredecessors(signal: any, predecessors: string[]) {
    await this.client.signal({
      ...signal,
      predecessors: [...new Set([...signal.predecessors, ...predecessors])],
    });
  }
}

// Bootstrap
async function main() {
  const client = await FluidicClient.connect({
    api: process.env.FLUIDIC_API_URL!,
    domain: "orderbook.eth-usdc",
  });

  const service = new OrderBookService(client, "orderbook.eth-usdc");
  service.on("error", console.error);
  await service.start();

  console.log("orderbook synthesizer running");
}

main();`}</code></pre>

            <h3>Rust: Direct Wave-Field Synthesis</h3>
            <p>
              For operators building custom nodes, the Rust API gives direct access to the oscillator.
            </p>
            <pre><code>{`use fluidic::consensus::Oscillator;
use fluidic::crypto::{AccountId, KeyPair, PhaseShift, StatefulShift, VectorClock};
use std::collections::HashMap;

fn main() {
    let oscillator = Oscillator::new([0u8; 32], 64);
    let operator = KeyPair::generate();
    let mut registry = HashMap::new();
    registry.insert(operator.account_id(), operator.public_key());

    oscillator.seed_account(operator.account_id(), 1_000_000_000_000);

    let mut vc = VectorClock::new();
    vc.tick(oscillator.id);

    let shift = StatefulShift::new(
        &operator,
        AccountId([1u8; 32]),
        1_000_000,
        vc,
        vec![],
        1,
        0,
    );

    oscillator.ingest(PhaseShift::Stateful(shift)).unwrap();
    let result = oscillator.synthesize(&registry);

    println!(
        "applied={} rejected={} burned={}",
        result.stateful_applied, result.stateful_rejected.len(), result.metabolic_burned
    );
}`}</code></pre>
          </Section>

          <Section id="evm" title="5. The EVM-Abstraction Layer">
            <h3>The Fluidic RPC Gateway</h3>
            <p>
              Existing wallets, Hardhat scripts, and Foundry deployments speak JSON-RPC. The <strong>Fluidic RPC Gateway</strong> translates that surface into mesh signals. From the caller’s perspective, nothing changes. Under the hood, every transaction becomes a causally-ordered signal inside the appropriate concurrency domain.
            </p>

            <h3>MetaMask / Wallet Configuration</h3>
            <table>
              <thead>
                <tr>
                  <th>Parameter</th>
                  <th>Value</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>Network Name</td>
                  <td>Fluidic Devnet</td>
                </tr>
                <tr>
                  <td>RPC URL</td>
                  <td><code>https://rpc.devnet.fluidic.network</code></td>
                </tr>
                <tr>
                  <td>Chain ID</td>
                  <td><code>1337</code></td>
                </tr>
                <tr>
                  <td>Currency Symbol</td>
                  <td>WAVE</td>
                </tr>
              </tbody>
            </table>

            <h3>Hardhat Deployment Script</h3>
            <pre><code>{`import { HardhatUserConfig } from "hardhat/config";

const config: HardhatUserConfig = {
  solidity: "0.8.19",
  networks: {
    fluidic: {
      url: "https://rpc.devnet.fluidic.network",
      accounts: [process.env.PRIVATE_KEY!],
      chainId: 1337,
    },
  },
};

export default config;`}</code></pre>

            <h3>Under the Hood: ERC-20 → Mesh Signal</h3>
            <p>
              When the gateway receives a standard <code>transfer</code> call, it:
            </p>
            <ol>
              <li>Parses the calldata into a structured signal payload.</li>
              <li>Derives the sender’s Fluidic account from the ECDSA signature.</li>
              <li>Injects a <code>token.transfer</code> signal into the asset’s concurrency domain.</li>
              <li>Returns a synthetic transaction hash immediately.</li>
              <li>Monitors the DAG for finalization and updates the JSON-RPC receipt when confirmed.</li>
            </ol>

            <pre><code>{`// Gateway-level signal produced from an ERC-20 transfer
{
  "type": "erc20.transfer",
  "domain": "asset:0xA0b86a33...",
  "from": "fluidic:0x71C7656EC7ab88b098defB751B7401B5f6d8976F",
  "to": "fluidic:0xdD870fA1b7C4700F2BD7f44238821C26f7392148",
  "amount": "1000000000000",
  "nonce": 42,
  "signature": "0x...",
  "predecessors": []
}`}</code></pre>

            <blockquote>
              The abstraction is bidirectional: Ethereum tooling sees a chain; Fluidic sees a signal. Developers keep their existing workflows while gaining horizontal scale and sub-millisecond synthesis.
            </blockquote>
          </Section>

          <footer className="mt-20 border-t border-white/10 pt-10">
            <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-[#8A8AA3]">
              FLUIDIC FOUNDATION // FED LABS — ARCHITECTS OF THE MESH
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
