import {
  FluidicClient,
  FluidicKeypair,
  submitSwap,
} from "@fluidic-foundation/sdk";

async function main() {
  const apiUrl = process.env.FLUIDIC_API_URL ?? "https://api.testnet.fluidic.foundation";
  const wallet = FluidicKeypair.generate();

  const client = new FluidicClient({
    apiUrl,
    minTick: "quorum",
  });

  console.log("account:", wallet.accountId);
  await client.register(wallet.publicKeyHex);
  console.log("faucet registered");

  const { poolInHash } = await submitSwap(client, {
    signer: wallet,
    direction: "WAVE_TO_USDC",
    amount: 1_000_000_000_000n, // 1 WAVE
    vectorClock: { entries: {} },
  });

  console.log("swap submitted:", poolInHash);
  const status = await client.waitForFinalization(poolInHash);
  console.log("swap finalized:", status.status);

  const balances = await client.balance(wallet.accountId);
  console.log("balances:", balances);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
