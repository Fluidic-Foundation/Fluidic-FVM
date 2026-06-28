import { FluidicClient, FluidicEvmProvider } from "@fluidic-foundation/sdk";
import { Wallet, parseEther } from "ethers";

async function main() {
  const apiUrl = process.env.FLUIDIC_API_URL ?? "https://api.testnet.fluidic.foundation";
  const ethWallet = Wallet.fromPhrase(
    process.env.MNEMONIC ?? "test test test test test test test test test test test junk"
  );

  const client = new FluidicClient({
    apiUrl,
    minTick: "quorum",
  });

  const evm = new FluidicEvmProvider(client);

  const tx = await ethWallet.populateTransaction({
    to: "0x3535353535353535353535353535353535353535",
    value: parseEther("0.01"),
    nonce: Number(await evm.getTransactionCount(ethWallet.address)),
    chainId: Number(await evm.chainId()),
  });

  const signed = await ethWallet.signTransaction(tx);
  const hash = await evm.sendRawTransaction(signed);
  console.log("sent:", hash);

  const receipt = await evm.waitForReceipt(hash);
  console.log("receipt:", receipt);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
