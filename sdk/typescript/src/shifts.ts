import type {
  AccountId,
  CommutativeShift,
  Coordinate,
  DomainId,
  PoolId,
  RegistrationShift,
  StakeShift,
  StatefulShift,
  TxHash,
  VectorClock,
} from "./types.js";
import { FluidicKeypair } from "./crypto.js";
import {
  bytesToHex,
  concatBytes,
  coordinateToBytes,
  ensureHex32,
  hashBlake3,
  hexToBytes,
  i128ToLeBytes,
  nowNs,
  u128ToLeBytes,
  u64ToLeBytes,
} from "./utils.js";

export const DEFAULT_DEX_DOMAIN: DomainId =
  "4445585f574156455f5553444300000000000000000000000000000000000000";

function vectorClockBytes(vc: VectorClock): Uint8Array {
  const entries = Object.entries(vc.entries)
    .map(([node, time]) => ({ node: ensureHex32(node), time }))
    .sort((a, b) => (a.node > b.node ? 1 : -1));
  const parts: Uint8Array[] = [];
  for (const { node, time } of entries) {
    parts.push(hexToBytes(node));
    parts.push(u64ToLeBytes(time));
  }
  return concatBytes(...parts);
}

export interface CommutativeShiftParams {
  signer: FluidicKeypair;
  coordinate: Coordinate;
  delta: bigint;
  poolId: PoolId;
  domain?: DomainId;
  nonce?: bigint;
  timestampNs?: bigint;
}

export function buildCommutativeShift(
  params: CommutativeShiftParams
): CommutativeShift {
  const domain = params.domain ?? DEFAULT_DEX_DOMAIN;
  const from = params.signer.accountId;
  const nonce = params.nonce ?? 0n;
  const timestampNs = params.timestampNs ?? nowNs();

  const signingBytes = concatBytes(
    new TextEncoder().encode("FLUIDIC:COMMUTATIVE:v3"),
    hexToBytes(ensureHex32(domain)),
    hexToBytes(ensureHex32(from)),
    coordinateToBytes(params.coordinate),
    i128ToLeBytes(params.delta),
    hexToBytes(ensureHex32(params.poolId)),
    u64ToLeBytes(nonce),
    u64ToLeBytes(timestampNs)
  );

  const signature = params.signer.sign(signingBytes);

  return {
    domain,
    from,
    coordinate: params.coordinate,
    delta: params.delta,
    pool_id: params.poolId,
    nonce,
    timestamp_ns: timestampNs,
    signature: bytesToHex(signature),
  };
}

export interface StatefulShiftParams {
  signer: FluidicKeypair;
  to: AccountId;
  amount: bigint;
  vectorClock: VectorClock;
  predecessors?: TxHash[];
  domain?: DomainId;
  nonce?: bigint;
  timestampNs?: bigint;
  /** Optional explicit sender account; defaults to the signer's main account. */
  from?: AccountId;
}

export function buildStatefulShift(
  params: StatefulShiftParams
): StatefulShift {
  const from = params.from ?? params.signer.accountId;
  const domain = params.domain ?? DEFAULT_DEX_DOMAIN;
  const predecessors = params.predecessors ?? [];
  const nonce = params.nonce ?? 0n;
  const timestampNs = params.timestampNs ?? nowNs();

  const signingBytes = concatBytes(
    new TextEncoder().encode("FLUIDIC:STATEFUL:v2"),
    hexToBytes(ensureHex32(domain)),
    hexToBytes(ensureHex32(from)),
    hexToBytes(ensureHex32(params.to)),
    u128ToLeBytes(params.amount),
    vectorClockBytes(params.vectorClock),
    concatBytes(...predecessors.map((h) => hexToBytes(ensureHex32(h)))),
    u64ToLeBytes(nonce),
    u64ToLeBytes(timestampNs)
  );

  const signature = params.signer.sign(signingBytes);

  return {
    domain,
    from,
    to: params.to,
    amount: params.amount,
    vector_clock: params.vectorClock,
    predecessors,
    nonce,
    timestamp_ns: timestampNs,
    signature: bytesToHex(signature),
  };
}

export interface RegistrationShiftParams {
  signer: FluidicKeypair;
  nonce?: bigint;
  timestampNs?: bigint;
}

export function buildRegistrationShift(
  params: RegistrationShiftParams
): RegistrationShift {
  return {
    account: params.signer.accountId,
    public_key: bytesToHex(params.signer.publicKey),
    wave_account: params.signer.waveAccount,
    usdc_account: params.signer.usdcAccount,
    nonce: params.nonce ?? 0n,
    timestamp_ns: params.timestampNs ?? nowNs(),
  };
}

export interface StakeShiftParams {
  signer: FluidicKeypair;
  amount: bigint;
  nonce?: bigint;
  timestampNs?: bigint;
}

export function buildStakeShift(params: StakeShiftParams): StakeShift {
  const operator = params.signer.accountId;
  const publicKey = bytesToHex(params.signer.publicKey);
  const nonce = params.nonce ?? 0n;
  const timestampNs = params.timestampNs ?? nowNs();

  const signingBytes = concatBytes(
    new TextEncoder().encode("FLUIDIC:STAKE:v1"),
    hexToBytes(ensureHex32(operator)),
    hexToBytes(ensureHex32(publicKey)),
    u128ToLeBytes(params.amount),
    u64ToLeBytes(nonce),
    u64ToLeBytes(timestampNs)
  );

  const signature = params.signer.sign(signingBytes);

  return {
    operator,
    public_key: publicKey,
    amount: params.amount,
    nonce,
    timestamp_ns: timestampNs,
    signature: bytesToHex(signature),
  };
}

export function hashStatefulShift(shift: StatefulShift): TxHash {
  const signingBytes = concatBytes(
    new TextEncoder().encode("FLUIDIC:STATEFUL:v2"),
    hexToBytes(ensureHex32(shift.domain)),
    hexToBytes(ensureHex32(shift.from)),
    hexToBytes(ensureHex32(shift.to)),
    u128ToLeBytes(shift.amount),
    vectorClockBytes(shift.vector_clock),
    concatBytes(...shift.predecessors.map((h) => hexToBytes(ensureHex32(h)))),
    u64ToLeBytes(shift.nonce),
    u64ToLeBytes(shift.timestamp_ns)
  );
  const sig = hexToBytes(shift.signature);
  return bytesToHex(hashBlake3(concatBytes(signingBytes, sig)));
}

export function hashCommutativeShift(shift: CommutativeShift): TxHash {
  const signingBytes = concatBytes(
    new TextEncoder().encode("FLUIDIC:COMMUTATIVE:v3"),
    hexToBytes(ensureHex32(shift.domain)),
    hexToBytes(ensureHex32(shift.from)),
    coordinateToBytes(shift.coordinate),
    i128ToLeBytes(shift.delta),
    hexToBytes(ensureHex32(shift.pool_id)),
    u64ToLeBytes(shift.nonce),
    u64ToLeBytes(shift.timestamp_ns)
  );
  const sig = hexToBytes(shift.signature);
  return bytesToHex(hashBlake3(concatBytes(signingBytes, sig)));
}
