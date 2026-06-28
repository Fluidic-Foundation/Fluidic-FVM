# @fluidic-foundation/sdk

TypeScript SDK for the Fluidic blockless wave-field mesh.

## Install

```bash
npm install @fluidic-foundation/sdk
```

For EVM wallet support also install the peer dependency:

```bash
npm install ethers
```

## Quick start

```ts
import { FluidicClient, FluidicKeypair, buildStatefulShift } from "@fluidic-foundation/sdk";

const wallet = FluidicKeypair.generate();

const client = new FluidicClient({
  apiUrl: "https://api.testnet.fluidic.foundation",
  minTick: "quorum", // auto-wait for the latest quorum-certified tick on reads
});

// Register / faucet (testnet only)
await client.register(wallet.publicKeyHex);

// Build and send a stateful transfer.
// The vector-clock entry must be for the sender account (`signer.accountId`
// here) and start at 1 for the first shift from that account.
const shift = buildStatefulShift({
  signer: wallet,
  to: recipientAccountId,
  amount: 1_000_000_000n,
  vectorClock: { entries: { [wallet.accountId]: 1n } },
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
import { submitSwap } from "@fluidic-foundation/sdk";

// For pool swaps the sender is the token account, so the vector-clock entry
// must use `wallet.waveAccount` for WAVE→USDC or `wallet.usdcAccount` for
// USDC→WAVE.
const { poolInHash } = await submitSwap(client, {
  signer: wallet,
  direction: "WAVE_TO_USDC",
  amount: 5_000_000_000_000n,
  vectorClock: { entries: { [wallet.waveAccount]: 1n } },
});

await client.waitForFinalization(poolInHash);
```

## EVM RPC bridge

```ts
import { FluidicEvmProvider } from "@fluidic-foundation/sdk/evm";

const evm = new FluidicEvmProvider(client);
const balance = await evm.getBalance("0x...");
```

## License

MIT
