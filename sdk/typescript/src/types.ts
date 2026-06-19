export type Hex = string;

/** 32-byte account identifier. */
export type AccountId = Hex;

/** 32-byte concurrency domain identifier. */
export type DomainId = Hex;

/** 32-byte liquidity pool identifier. */
export type PoolId = Hex;

/** 32-byte transaction/signal hash. */
export type TxHash = Hex;

/** 32-byte oscillator node identifier. */
export type OscillatorId = Hex;

/** 20-byte EVM address. */
export type EvmAddress = Hex;

/** Ed25519 public key, 32 bytes hex. */
export type PublicKey = Hex;

/** A multi-dimensional frequency coordinate. */
export interface Coordinate {
  components: [bigint, bigint, bigint, bigint];
}

/** Vector clock used for stateful ordering. */
export interface VectorClock {
  entries: Record<OscillatorId, bigint>;
}

/** Raw commutative shift as accepted by the node. */
export interface CommutativeShift {
  domain: DomainId;
  coordinate: Coordinate;
  delta: bigint;
  pool_id: PoolId;
  nonce: bigint;
  timestamp_ns: bigint;
  first_seen_at_ns?: bigint;
  signature: Hex;
}

/** Raw stateful shift as accepted by the node. */
export interface StatefulShift {
  domain: DomainId;
  from: AccountId;
  to: AccountId;
  amount: bigint;
  vector_clock: VectorClock;
  predecessors: TxHash[];
  nonce: bigint;
  timestamp_ns: bigint;
  first_seen_at_ns?: bigint;
  signature: Hex;
}

/** Registration event gossiped to the mesh. */
export interface RegistrationShift {
  account: AccountId;
  public_key: PublicKey;
  wave_account: AccountId;
  usdc_account: AccountId;
  nonce: bigint;
  timestamp_ns: bigint;
}

/** Stake event gossiped to the mesh. */
export interface StakeShift {
  operator: AccountId;
  public_key: PublicKey;
  amount: bigint;
  nonce: bigint;
  timestamp_ns: bigint;
  signature: Hex;
}

export interface StateResponse {
  wave_reserve: string;
  usdc_reserve: string;
  price: number;
  throughput: number;
  latency_ms: number;
  metabolic_burned: string;
  commutative_applied: number;
  stateful_applied: number;
  evm_applied: number;
  pool_wave_account: Hex;
  pool_usdc_account: Hex;
}

export interface BalanceResponse {
  wave: string;
  usdc: string;
}

export interface ShiftStatusResponse {
  hash: Hex;
  status: "unknown" | "accepted" | "finalized" | "rejected";
  error: string | null;
  synthesis_tick: number;
  confirmations: number;
}

export interface QuorumView {
  commutative: Hex;
  stateful: Hex;
  evm: Hex;
  balances: Hex;
  stake: Hex;
  reward: Hex;
}

export interface QuorumResponse {
  tick: number;
  finalized: boolean;
  stake?: string;
  threshold: string;
  total_stake: string;
  roots?: QuorumView;
}

export interface OperatorInfo {
  account: AccountId;
  public_key: PublicKey;
  stake: string;
  min_stake: string;
  is_staked: boolean;
}

export interface StateSnapshot {
  wave_reserve: string;
  usdc_reserve: string;
  price: number;
  throughput: number;
  latency_ms: number;
  network_ms: number;
  metabolic_burned: string;
  commutative_applied: number;
  stateful_applied: number;
  evm_applied: number;
  pool_wave_account: string;
  pool_usdc_account: string;
  accounts?: Record<string, string>;
}

export interface FluidicClientOptions {
  /** Base URL of the Fluidic API, e.g. https://api.testnet.fluidic.foundation */
  apiUrl: string;
  /** WebSocket URL; defaults to replacing http(s):// with ws(s):// + /api/ws */
  wsUrl?: string;
  /** Default minimum synthesis tick to wait for on reads. */
  minTick?: "none" | "latest" | "quorum" | number;
  /** Request timeout in milliseconds. */
  timeoutMs?: number;
  /** Number of retries for idempotent reads. */
  retries?: number;
}

export interface SwapQuote {
  amountIn: bigint;
  amountOut: bigint;
  priceImpact: number;
}
