"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function ShiftsPage() {
  return (
    <DocPage title="Shifts">
      <p>
        A <strong>shift</strong> is the fundamental unit of work in Fluidic. Shifts are signed state transitions submitted by accounts. They come in two forms: <strong>stateful</strong> shifts, which require causal ordering, and <strong>commutative</strong> shifts, which can be applied in any order.
      </p>

      <h2>Commutative vs. stateful shifts</h2>
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

      <h2>Stateful shifts</h2>
      <p>
        Stateful shifts need causal ordering (transfers, EVM transactions). They go into the vector-clock DAG, exactly like Ethereum transactions need a total order. Each stateful shift carries a vector clock and a nonce so the node can detect double-spends and enforce happens-before relationships.
      </p>

      <h2>Commutative shifts</h2>
      <p>
        Commutative shifts are order-independent (AMM pool deltas, streaming payments). They are batched with an NTT, similar to how Solana parallelizes non-conflicting instructions. Because addition is commutative, the node can sum deltas in a batch window without worrying about ordering.
      </p>

      <h2>Concurrency domains</h2>
      <p>
        A <strong>domain</strong> is a 32-byte scope tag (e.g., <code>DEFAULT_DEX_DOMAIN</code>). Commutative shifts within the same domain are batched together; stateful shifts carry a domain and are validated against that domain&apos;s DAG. Domains allow many independent applications to share the mesh without contending for a global lock.
      </p>

      <h2>Comparison with other chains</h2>
      <p>
        On Ethereum every transaction is linearly ordered. On Solana, instructions within a transaction can execute in parallel if they touch disjoint accounts. Fluidic makes that distinction explicit at the protocol level.
      </p>

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/core-concepts/synthesis-ticks">Synthesis Ticks</Link> — how shifts become state.</li>
        <li><Link href="/docs/core-concepts/accounts">Accounts</Link> — who can submit shifts.</li>
        <li><Link href="/docs/api-reference/typescript-sdk">TypeScript SDK</Link> — building and signing shifts.</li>
      </ul>
    </DocPage>
  );
}
