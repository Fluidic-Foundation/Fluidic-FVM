"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function SynthesisTicksPage() {
  return (
    <DocPage title="Synthesis Ticks">
      <p>
        A <strong>synthesis tick</strong> is a periodic state transition. At each tick the oscillator applies metabolic burn, drains pending queues into the DAG, finalizes previous shifts, applies commutative batches, executes EVM transactions, and computes Merkle roots.
      </p>

      <h2>The oscillator</h2>
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

      <h2>Synthesis certificate fields</h2>
      <p>
        Every tick produces a <strong>Synthesis Certificate</strong>: a signed bundle that makes the state reproducible and auditable.
      </p>
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

      <h2>Concurrency domains</h2>
      <p>
        A <strong>domain</strong> is a 32-byte scope tag (e.g., <code>DEFAULT_DEX_DOMAIN</code>). Commutative shifts within the same domain are batched together; stateful shifts carry a domain and are validated against that domain&apos;s DAG. Domains allow many independent applications to share the mesh without contending for a global lock.
      </p>

      <h2>Networking</h2>
      <p>
        Nodes connect over TCP gossip. Each node binds a gossip socket (default <code>0.0.0.0:7000</code>) and dials the comma-separated <code>PEERS</code>. Signed shifts, registrations, stakes, and certificates are propagated through the mesh.
      </p>

      <h2>Node configuration</h2>
      <table>
        <thead>
          <tr><th>Variable</th><th>Default</th><th>Description</th></tr>
        </thead>
        <tbody>
          <tr><td>OSCILLATOR_ID</td><td>0</td><td>Node identity; must be a number or end with one</td></tr>
          <tr><td>API_PORT</td><td>8080</td><td>HTTP/WebSocket API port</td></tr>
          <tr><td>BIND_ADDR</td><td>0.0.0.0:7000</td><td>TCP gossip bind address</td></tr>
          <tr><td>PEERS</td><td>&quot;34.56.159.76:7000&quot;</td><td>Testnet gossip seed (comma-separated list supported)</td></tr>
          <tr><td>SYNTHESIS_INTERVAL_MS</td><td>1000</td><td>Tick interval</td></tr>
        </tbody>
      </table>

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/core-concepts/consensus-staking">Consensus &amp; Staking</Link> — how certificates become final.</li>
        <li><Link href="/docs/core-concepts/shifts">Shifts</Link> — the inputs to each tick.</li>
        <li><Link href="/docs/api-reference/rest-api">REST API</Link> — querying ticks and certificates.</li>
      </ul>
    </DocPage>
  );
}
