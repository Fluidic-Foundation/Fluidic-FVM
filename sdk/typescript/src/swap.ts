import type { AccountId, SwapQuote, VectorClock } from "./types.js";
import { FluidicKeypair } from "./crypto.js";
import {
  buildStatefulShift,
  DEFAULT_DEX_DOMAIN,
  hashStatefulShift,
} from "./shifts.js";
import type { FluidicClient } from "./client.js";

export type SwapDirection = "WAVE_TO_USDC" | "USDC_TO_WAVE";

export interface SwapParams {
  signer: FluidicKeypair;
  direction: SwapDirection;
  amount: bigint;
  /** Vector clock to attach to the swap shift. */
  vectorClock: VectorClock;
  /** Optional predecessor hashes for causal ordering. */
  predecessors?: string[];
  nonce?: bigint;
  timestampNs?: bigint;
}

/**
 * Build and submit a pool swap. A swap is two stateful shifts:
 *   1. user token account -> pool token account
 *   2. pool token account -> user receiving token account (payout)
 *
 * The SDK computes the payout amount using the same integer arithmetic as the
 * node so the submitted payout shift matches what the node will produce.
 */
export async function submitSwap(
  client: FluidicClient,
  params: SwapParams
): Promise<{ poolInHash: string; payoutHash: string }> {
  const state = await client.state();
  const poolWaveAccount = state.pool_wave_account;
  const poolUsdcAccount = state.pool_usdc_account;

  const waveReserve = BigInt(state.wave_reserve);
  const usdcReserve = BigInt(state.usdc_reserve);
  if (waveReserve === 0n || usdcReserve === 0n) {
    throw new Error("Pool reserves are empty");
  }

  const mainAccount = params.signer.accountId;
  const waveUser = params.signer.waveAccount;
  const usdcUser = params.signer.usdcAccount;

  let from: AccountId;
  let to: AccountId;
  let payoutFrom: AccountId;
  let payoutTo: AccountId;
  let payoutAmount: bigint;

  if (params.direction === "WAVE_TO_USDC") {
    from = waveUser;
    to = poolWaveAccount;
    payoutFrom = poolUsdcAccount;
    payoutTo = usdcUser;
    payoutAmount = (params.amount * usdcReserve) / waveReserve;
  } else {
    from = usdcUser;
    to = poolUsdcAccount;
    payoutFrom = poolWaveAccount;
    payoutTo = waveUser;
    payoutAmount = (params.amount * waveReserve) / usdcReserve;
  }

  if (payoutAmount === 0n) {
    throw new Error("Swap output is zero");
  }

  const common = {
    domain: DEFAULT_DEX_DOMAIN,
    vectorClock: params.vectorClock,
    predecessors: params.predecessors ?? [],
    nonce: params.nonce ?? 0n,
    timestampNs: params.timestampNs,
  };

  const poolIn = buildStatefulShift({
    signer: params.signer,
    from,
    to,
    amount: params.amount,
    ...common,
  });

  // The payout shift must be signed by the pool. In the current testnet the
  // pool keypair is held server-side, so the API constructs the payout shift
  // automatically when it sees a shift targeting the pool. We therefore only
  // submit the user-side shift and return its hash.
  //
  // Future versions will allow a local pool keypair to build both shifts here.

  await client.submitStateful(poolIn);
  return {
    poolInHash: hashStatefulShift(poolIn),
    payoutHash: "", // server-generated; query via /api/shift/:hash/status
  };
}

export function quoteSwap(
  direction: SwapDirection,
  amount: bigint,
  waveReserve: bigint,
  usdcReserve: bigint
): SwapQuote {
  if (waveReserve === 0n || usdcReserve === 0n) {
    throw new Error("Pool reserves are empty");
  }
  const amountOut =
    direction === "WAVE_TO_USDC"
      ? (amount * usdcReserve) / waveReserve
      : (amount * waveReserve) / usdcReserve;
  const priceImpact = Number(amountOut) / Number(amount);
  return { amountIn: amount, amountOut, priceImpact };
}
