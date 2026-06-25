"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function AccountsPage() {
  return (
    <DocPage title="Accounts">
      <p>
        Fluidic uses <strong>Ed25519</strong> keypairs. A public key is hashed to a 32-byte <strong>AccountId</strong>. From that account, the node derives token accounts (WAVE, USDC) using deterministic hashes of the account and asset tag.
      </p>

      <h2>Creating a wallet with the SDK</h2>
      <pre><code>{`import { FluidicKeypair } from "@fluidic/sdk";

const wallet = FluidicKeypair.generate();
console.log("account:", wallet.accountId);
console.log("public key:", wallet.publicKeyHex);`}</code></pre>

      <h2>Registering with a node</h2>
      <p>
        Before a node accepts stateful shifts from an account, the public key must be registered in its key registry. The <code>/api/account/register</code> endpoint records the key and seeds token accounts from a faucet.
      </p>
      <pre><code>{`const client = new FluidicClient({ apiUrl: "http://localhost:8080" });
await client.register(wallet.publicKeyHex);`}</code></pre>

      <h2>EVM account mapping</h2>
      <p>
        For EVM compatibility, any 20-byte Ethereum address is deterministically mapped to a 32-byte Fluidic account using <code>blake3(&quot;fluidic:evm-account:v1&quot; || address)</code>, so an EVM wallet&apos;s balance is readable through both RPCs.
      </p>
      <p>
        EVM contracts are just EVM accounts with code. They execute inside <strong>revm</strong>, the same interpreter used by Optimism, Arbitrum, and many L2s. Contract bytecode and storage persist across synthesis ticks, so you can deploy once and interact forever — exactly like on Ethereum.
      </p>

      <h2>Concept map</h2>
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
        </tbody>
      </table>

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/core-concepts/shifts">Shifts</Link> — what accounts can submit.</li>
        <li><Link href="/docs/api-reference/typescript-sdk">TypeScript SDK</Link> — keypair and client methods.</li>
        <li><Link href="/docs/core-concepts/evm-compatibility">EVM Compatibility</Link> — using Ethereum wallets.</li>
      </ul>
    </DocPage>
  );
}
