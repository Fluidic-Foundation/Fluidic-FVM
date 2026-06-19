import type { EvmAddress, TxHash } from "./types.js";
import type { FluidicClient } from "./client.js";
import { FLUIDIC_EVM_CHAIN_ID } from "./constants.js";

export { FLUIDIC_EVM_CHAIN_ID };

/** JSON-RPC provider that targets the Fluidic EVM gateway. */
export class FluidicEvmProvider {
  readonly client: FluidicClient;

  constructor(client: FluidicClient) {
    this.client = client;
  }

  private async rpc(method: string, params: unknown[]): Promise<unknown> {
    const query = this.client["minTickQuery"]() ?? "";
    const url = `${this.client.apiUrl}/rpc${query}`;
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method,
        params,
      }),
    });
    if (!res.ok) {
      throw new Error(`EVM RPC HTTP ${res.status}`);
    }
    const json = (await res.json()) as {
      result?: unknown;
      error?: { message: string; code: number };
    };
    if (json.error) {
      throw new Error(`EVM RPC ${json.error.code}: ${json.error.message}`);
    }
    return json.result;
  }

  async blockNumber(): Promise<bigint> {
    const result = await this.rpc("eth_blockNumber", []);
    return BigInt(String(result));
  }

  async chainId(): Promise<bigint> {
    const result = await this.rpc("eth_chainId", []);
    return BigInt(String(result));
  }

  async getBalance(address: EvmAddress): Promise<bigint> {
    const result = await this.rpc("eth_getBalance", [address, "latest"]);
    return BigInt(String(result));
  }

  async getTransactionCount(address: EvmAddress): Promise<bigint> {
    const result = await this.rpc("eth_getTransactionCount", [address, "latest"]);
    return BigInt(String(result));
  }

  async sendRawTransaction(signedTx: string): Promise<TxHash> {
    const result = await this.rpc("eth_sendRawTransaction", [signedTx]);
    return String(result);
  }

  async getTransactionReceipt(hash: TxHash): Promise<unknown> {
    return this.rpc("eth_getTransactionReceipt", [hash]);
  }

  async waitForReceipt(
    hash: TxHash,
    options: { pollMs?: number; timeoutMs?: number } = {}
  ): Promise<unknown> {
    const pollMs = options.pollMs ?? 500;
    const deadline = Date.now() + (options.timeoutMs ?? 60_000);
    while (Date.now() < deadline) {
      const receipt = await this.getTransactionReceipt(hash);
      if (receipt !== null && receipt !== undefined) return receipt;
      await new Promise((r) => setTimeout(r, pollMs));
    }
    throw new Error(`Timeout waiting for EVM receipt ${hash}`);
  }
}

/** Poll an EVM transaction receipt until it is no longer null. */
export async function waitForEvmReceipt(
  provider: FluidicEvmProvider,
  hash: TxHash,
  options: { pollMs?: number; timeoutMs?: number } = {}
): Promise<unknown> {
  return provider.waitForReceipt(hash, options);
}
