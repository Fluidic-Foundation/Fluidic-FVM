"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function RestApiPage() {
  return (
    <DocPage title="REST API">
      <p>
        The Fluidic node exposes an HTTP API on the port configured by <code>API_PORT</code> (default <code>8080</code>). A WebSocket stream is also available for live state snapshots.
      </p>

      <h2>Base URL</h2>
      <ul>
        <li>Local: <code>http://localhost:8080</code></li>
        <li>Testnet: <code>https://api.testnet.fluidic.foundation</code></li>
      </ul>

      <h2>State</h2>
      <ul>
        <li><code>GET /api/state</code> — pool reserves, price, throughput, applied counts.</li>
        <li><code>GET /api/ws</code> — WebSocket stream of state snapshots.</li>
      </ul>

      <h2>Accounts</h2>
      <ul>
        <li><code>POST /api/account/register</code> — register a public key, returns derived token accounts.</li>
        <li><code>GET /api/account/:id/balance</code> — WAVE/USDC balances.</li>
        <li><code>GET /api/operators</code> — list staked operators.</li>
      </ul>

      <h2>Shifts</h2>
      <ul>
        <li><code>POST /api/shift/stateful</code> — submit a signed stateful shift.</li>
        <li><code>POST /api/shift/commutative</code> — submit a commutative shift.</li>
        <li><code>GET /api/shift/:hash/status</code> — <code>unknown | accepted | finalized | rejected</code>.</li>
        <li><code>GET /api/shifts/recent?limit=N</code> — recent accepted shifts.</li>
      </ul>

      <h2>EVM</h2>
      <ul>
        <li><code>POST /api/evm/tx</code> — submit a signed raw Ethereum transaction.</li>
        <li><code>POST /api/evm/faucet</code> — fund an EVM address with WAVE.</li>
        <li><code>POST /rpc</code> — Ethereum JSON-RPC namespace (chain, balances, sendRaw, call, code, receipts).</li>
      </ul>

      <h2>Consensus</h2>
      <ul>
        <li><code>GET /api/certificate/:tick</code> — certificate for a tick.</li>
        <li><code>GET /api/quorum/:tick</code> — quorum status and signatures.</li>
        <li><code>GET /api/ticks/recent?limit=N</code> — recent synthesis ticks.</li>
        <li><code>GET /api/ticks/:tick</code> — single tick summary.</li>
      </ul>

      <h2>Operator</h2>
      <ul>
        <li><code>GET /api/operator/info</code> — local operator account/stake.</li>
        <li><code>POST /api/operator/stake</code> — stake additional WAVE.</li>
      </ul>

      <h2>Faucet</h2>
      <ul>
        <li><code>POST /api/faucet</code> — drip WAVE/USDC to a registered Fluidic account.</li>
      </ul>

      <h2>Full endpoint list</h2>
      <p>
        The public API exposes the following paths: <code>/api/state</code>, <code>/api/ticks/recent</code>, <code>/api/ticks/:tick</code>, <code>/api/shifts/recent</code>, <code>/api/operators</code>, <code>/api/account/:id</code>, <code>/api/account/register</code>, <code>/api/faucet</code>, <code>/api/transfer</code>, <code>/api/commutative/swap</code>, <code>/api/certificate/:tick</code>, <code>/api/quorum/:tick</code>, <code>/api/operator/info</code>, <code>/api/operator/stake</code>, <code>/api/evm/faucet</code>, <code>/rpc</code>, and <code>/api/ws</code>.
      </p>

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/api-reference/evm-rpc">EVM RPC</Link> — JSON-RPC methods.</li>
        <li><Link href="/docs/api-reference/typescript-sdk">TypeScript SDK</Link> — typed client wrappers.</li>
        <li><Link href="/docs/core-concepts/synthesis-ticks">Synthesis Ticks</Link> — what the consensus endpoints return.</li>
      </ul>
    </DocPage>
  );
}
