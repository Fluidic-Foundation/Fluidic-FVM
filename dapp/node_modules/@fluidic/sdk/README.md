# @fluidic/sdk

TypeScript SDK for the Fluidic blockless wave-field mesh.

## Install

```bash
npm install @fluidic/sdk
```

For EVM wallet support also install the peer dependency:

```bash
npm install ethers
```

## Quick start

```ts
import { FluidicClient, FluidicKeypair, buildStatefulShift } from "@fluidic/sdk";

const wallet = FluidicKeypair.generate();

const client = new FluidicClient({
  apiUrl: "https://api.testnet.fluidic.foundation",
  minTick: "quorum", // auto-wait for the latest quorum-certified tick on reads
});

// Register / faucet (testnet only)
await client.register(wallet.publicKeyHex);

// Build and send a stateful transfer
const shift = buildStatefulShift({
  signer: wallet,
  to: recipientAccountId,
  amount: 1_000_000_000n,
  vectorClock: { entries: {} },
});

const { hash } = await client.submitStateful(shift);
await client.waitForFinalization(hash);
```

## Read consistency

Fluidic nodes are load-balanced without sticky sessions. Every read endpoint
accepts `?min_tick=` so clients can avoid stale responses:

```ts
const client = new FluidicClient({
  apiUrl: "...",
  minTick: "quorum", // attaches the latest known quorum tick automatically
});
```

Modes:
- `"none"` — no waiting (default)
- `"latest"` — wait for the highest local tick seen by the SDK
- `"quorum"` — wait for the highest quorum-finalized tick seen by the SDK
- `number` — wait for a specific tick

## Pool swaps

```ts
import { submitSwap } from "@fluidic/sdk";

const { poolInHash } = await submitSwap(client, {
  signer: wallet,
  direction: "WAVE_TO_USDC",
  amount: 5_000_000_000_000n,
  vectorClock: { entries: {} },
});

await client.waitForFinalization(poolInHash);
```

## EVM RPC bridge

```ts
import { FluidicEvmProvider } from "@fluidic/sdk/evm";

const evm = new FluidicEvmProvider(client);
const balance = await evm.getBalance("0x...");
```

## License

MIT
