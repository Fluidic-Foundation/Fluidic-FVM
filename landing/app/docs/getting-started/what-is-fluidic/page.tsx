"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function WhatIsFluidicPage() {
  return (
    <DocPage title="What is Fluidic?">
      <p>
        Fluidic is a <strong>blockless state-synthesis network</strong>. Instead of collecting transactions into blocks and ordering them through a leader, Fluidic nodes continuously ingest signed <strong>shifts</strong>, order stateful shifts in a vector-clock DAG, batch commutative shifts through Number-Theoretic Transforms, and synthesize the resulting state in periodic ticks.
      </p>
      <p>
        Every tick produces a <strong>Synthesis Certificate</strong>: a signed bundle containing Merkle roots of the commutative state, stateful DAG, balances, stake table, reward pool, and EVM transactions. Operators sign certificates, and once a quorum of stake-weighted signatures is observed, the tick is considered finalized.
      </p>

      <h2>Core ideas</h2>
      <ul>
        <li><strong>No blocks.</strong> State advances continuously through synthesis ticks.</li>
        <li><strong>No mempool auction.</strong> Shifts are gossiped and causally ordered, not front-run.</li>
        <li><strong>Parallel by default.</strong> Commutative shifts merge in NTT windows; stateful shifts merge through the DAG.</li>
        <li><strong>Permissionless.</strong> Anyone can run a synthesis node and earn rewards.</li>
        <li><strong>EVM-compatible.</strong> Raw Ethereum transactions execute inside a revm sandbox.</li>
      </ul>

      <h2>Where to go next</h2>
      <ul>
        <li><Link href="/docs/getting-started/quickstart">Quickstart</Link> — run a node and submit a swap.</li>
        <li><Link href="/docs/core-concepts/shifts">Shifts</Link> — learn the difference between stateful and commutative operations.</li>
        <li><Link href="/docs/core-concepts/synthesis-ticks">Synthesis Ticks</Link> — how the oscillator turns shifts into finalized state.</li>
        <li><Link href="/docs/tutorials/deploy-contract">Deploy a Contract</Link> — deploy Solidity on the testnet.</li>
      </ul>
    </DocPage>
  );
}
