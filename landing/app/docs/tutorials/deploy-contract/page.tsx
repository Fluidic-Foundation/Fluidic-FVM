"use client";

import Link from "next/link";
import { DocPage } from "../../components/doc-page";

export default function DeployContractPage() {
  return (
    <DocPage title="Tutorial: Deploy a Contract">
      <p>
        This tutorial deploys a simple <code>Counter</code> Solidity contract on the Fluidic testnet. The steps are the same for any Base Sepolia contract: fund the EVM wallet, compile with your normal toolchain, and deploy to the Fluidic RPC.
      </p>

      <h2>Prerequisites</h2>
      <ul>
        <li><a href="https://book.getfoundry.sh" target="_blank" rel="noreferrer">Foundry</a> installed (<code>forge</code>, <code>cast</code>).</li>
        <li>An EVM private key. This tutorial uses the standard Ganache test key <code>0x4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d</code> (address <code>0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1</code>). Do not use this key for real funds.</li>
      </ul>

      <h2>1. Create the project</h2>
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

      <h2>2. Fund the deployer</h2>
      <pre><code>{`export PK=0x4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d
export ADDR=$(cast wallet address --private-key $PK)

curl -X POST https://api.testnet.fluidic.foundation/api/evm/faucet \\
  -H "Content-Type: application/json" \\
  -d "{\"address\":\"$ADDR\"}"

# Check the mapped Fluidic balance
cast balance --rpc-url https://api.testnet.fluidic.foundation/rpc $ADDR`}</code></pre>

      <h2>3. Deploy</h2>
      <pre><code>{`forge create src/Counter.sol:Counter \\
  --rpc-url https://api.testnet.fluidic.foundation/rpc \\
  --private-key $PK \\
  --chain 990492 \\
  --legacy \\
  --broadcast`}</code></pre>

      <h2>4. Verify deployment</h2>
      <p>
        If the transaction succeeds, Forge prints the deployed address. You can read it with <code>eth_getCode</code>. The deployed Counter example used in this documentation lives at:
      </p>
      <pre><code>{`cast code --rpc-url https://api.testnet.fluidic.foundation/rpc 0xC89Ce4735882C9F0f0FE26686c53074E09B0D550`}</code></pre>

      <h2>5. Read and write state</h2>
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

      <h2>6. Bring a Base Sepolia contract across</h2>
      <p>
        To migrate an existing Base Sepolia contract, copy its source into the same Foundry project, keep the same compiler version, and run the same <code>forge create</code> command. The contract will receive a deterministic address on Fluidic based on the deployer nonce. If you need the exact same address as on Base Sepolia, use the same deployer account and nonce; otherwise treat Fluidic as a fresh deployment and update your frontend&apos;s contract addresses.
      </p>
      <p>
        For contracts that read Base Sepolia state (e.g., an oracle or bridge), run a relayer that pushes state into Fluidic.
      </p>

      <h2>Related topics</h2>
      <ul>
        <li><Link href="/docs/core-concepts/evm-compatibility">EVM Compatibility</Link> — network details and RPC methods.</li>
        <li><Link href="/docs/tutorials/build-dapp">Build a dApp</Link> — connect a frontend to your contract.</li>
        <li><Link href="/docs/getting-started/testnet">Testnet</Link> — faucet and public RPC.</li>
      </ul>
    </DocPage>
  );
}
