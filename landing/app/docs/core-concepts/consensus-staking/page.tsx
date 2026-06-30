"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function ConsensusStakingPage() {
  return (
    <DocPage title="Consensus & Staking">
      <p>
        Fluidic reaches finality through stake-weighted BFT certificates. Anyone can run a node, but only staked operators sign synthesis certificates and earn rewards.
      </p>

      <h2>Validators & staking</h2>
      <p>
        A node becomes a validator when its operator account is <strong>staked</strong>. On first boot the node seeds a genesis balance and stakes it automatically. Additional stake can be added through <code>/api/operator/stake</code>.
      </p>
      <h3>Minimum stake</h3>
      <p>
        The default minimum stake is <code>1e18</code> units. A node whose operator stake is below this threshold can still run and ingest shifts, but it will not sign synthesis certificates.
      </p>

      <h2>Rewards</h2>
      <p>
        Every synthesis tick applies metabolic decay. Of the decayed value, a fixed share (25%) is permanently burned — reducing circulating supply — and the remaining 75% is distributed to staked operators proportional to their stake and to the liquidity-provider reward pool. Operator rewards accrue in the reward pool and can be claimed via the stake table.
      </p>

      <h2>Quorum & finality</h2>
      <p>
        Finality is BFT. A tick is finalized when the <code>CertificateTracker</code> observes signatures from operators holding at least <code>2/3 + 1</code> of total stake. Conflicting certificates for the same tick are detected and the offending operator is slashed.
      </p>
      <p>
        Stateful shifts reach <code>finalized</code> status after surviving <code>FINALIZATION_DEPTH</code> synthesis ticks without a conflicting double-spend being accepted into the DAG.
      </p>

      <h2>Tokenomics</h2>
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

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/core-concepts/synthesis-ticks">Synthesis Ticks</Link> — what operators sign.</li>
        <li><Link href="/docs/getting-started/quickstart">Quickstart</Link> — run a staked node locally.</li>
        <li><Link href="/docs/api-reference/rest-api">REST API</Link> — operator and certificate endpoints.</li>
      </ul>
    </DocPage>
  );
}
