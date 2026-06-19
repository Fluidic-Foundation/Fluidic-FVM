import { blake3 } from "@noble/hashes/blake3";

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  if (clean.length % 2 !== 0) {
    throw new Error(`Invalid hex length: ${hex}`);
  }
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((acc, p) => acc + p.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const p of parts) {
    out.set(p, offset);
    offset += p.length;
  }
  return out;
}

export function u64ToLeBytes(n: bigint): Uint8Array {
  const buf = new Uint8Array(8);
  for (let i = 0; i < 8; i++) {
    buf[i] = Number((n >> BigInt(i * 8)) & BigInt(0xff));
  }
  return buf;
}

export function u128ToLeBytes(n: bigint): Uint8Array {
  const buf = new Uint8Array(16);
  for (let i = 0; i < 16; i++) {
    buf[i] = Number((n >> BigInt(i * 8)) & BigInt(0xff));
  }
  return buf;
}

export function i128ToLeBytes(n: bigint): Uint8Array {
  // Two's complement in 128 bits.
  let value = n % BigInt(2) ** BigInt(128);
  if (value < 0n) value += BigInt(2) ** BigInt(128);
  const buf = new Uint8Array(16);
  for (let i = 0; i < 16; i++) {
    buf[i] = Number((value >> BigInt(i * 8)) & BigInt(0xff));
  }
  return buf;
}

export function leBytesToU128(bytes: Uint8Array): bigint {
  let n = 0n;
  for (let i = 0; i < bytes.length; i++) {
    n += BigInt(bytes[i]) << BigInt(i * 8);
  }
  return n;
}

export function padHex32(hex: string): string {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  if (clean.length > 64) throw new Error("hex longer than 32 bytes");
  return clean.padStart(64, "0");
}

export function ensureHex32(value: string): string {
  const clean = padHex32(value);
  return clean.toLowerCase();
}

export function hashBlake3(data: Uint8Array): Uint8Array {
  return blake3(data);
}

export function accountIdFromPublicKey(publicKey: Uint8Array): string {
  return bytesToHex(blake3(publicKey));
}

export function deriveAccount(base: string, salt: string): string {
  return bytesToHex(
    blake3(
      concatBytes(
        new TextEncoder().encode("fluidic:derived-account:v1"),
        hexToBytes(ensureHex32(base)),
        new TextEncoder().encode(salt)
      )
    )
  );
}

export function waveAddressFromAccountId(accountId: string): string {
  return bytesToHex(
    blake3(
      concatBytes(
        new TextEncoder().encode("fluidic:wave-address:v1"),
        hexToBytes(ensureHex32(accountId))
      )
    )
  );
}

export function evmAddressToFluidicAccount(evmAddress: string): string {
  const clean = evmAddress.startsWith("0x") ? evmAddress.slice(2) : evmAddress;
  if (clean.length !== 40) throw new Error("EVM address must be 20 bytes");
  return bytesToHex(
    blake3(
      concatBytes(
        new TextEncoder().encode("fluidic:evm-account:v1"),
        hexToBytes(clean)
      )
    )
  );
}

export function coordinateToBytes(coord: {
  components: [bigint, bigint, bigint, bigint];
}): Uint8Array {
  const buf = new Uint8Array(32);
  for (let i = 0; i < 4; i++) {
    const c = coord.components[i];
    for (let j = 0; j < 8; j++) {
      buf[i * 8 + j] = Number((c >> BigInt(j * 8)) & BigInt(0xff));
    }
  }
  return buf;
}

export function nowNs(): bigint {
  return BigInt(Date.now()) * 1_000_000n;
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
