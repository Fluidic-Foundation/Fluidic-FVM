import { ed25519 } from "@noble/curves/ed25519";
import {
  accountIdFromPublicKey,
  bytesToHex,
  deriveAccount,
  ensureHex32,
  hexToBytes,
  waveAddressFromAccountId,
} from "./utils.js";

/** Fluidic Ed25519 keypair. */
export class FluidicKeypair {
  /** 32-byte private key / seed. */
  readonly secretKey: Uint8Array;
  /** 32-byte public key. */
  readonly publicKey: Uint8Array;

  constructor(secretKey: Uint8Array, publicKey: Uint8Array) {
    this.secretKey = new Uint8Array(secretKey);
    this.publicKey = new Uint8Array(publicKey);
  }

  /** Generate a random keypair. */
  static generate(): FluidicKeypair {
    const priv = ed25519.utils.randomPrivateKey();
    const pub = ed25519.getPublicKey(priv);
    return new FluidicKeypair(priv, pub);
  }

  /** Derive a keypair from a 32-byte seed. */
  static fromSeed(seed: Uint8Array | string): FluidicKeypair {
    const bytes =
      typeof seed === "string" ? hexToBytes(ensureHex32(seed)) : seed;
    if (bytes.length !== 32) {
      throw new Error("Seed must be 32 bytes");
    }
    const pub = ed25519.getPublicKey(bytes);
    return new FluidicKeypair(bytes, pub);
  }

  /** Reconstruct from a secret key (public key is derived). */
  static fromSecretKey(secretKey: Uint8Array | string): FluidicKeypair {
    const bytes =
      typeof secretKey === "string"
        ? hexToBytes(ensureHex32(secretKey))
        : secretKey;
    const pub = ed25519.getPublicKey(bytes);
    return new FluidicKeypair(bytes, pub);
  }

  get publicKeyHex(): string {
    return bytesToHex(this.publicKey);
  }

  get secretKeyHex(): string {
    return bytesToHex(this.secretKey);
  }

  get accountId(): string {
    return accountIdFromPublicKey(this.publicKey);
  }

  get waveAddress(): string {
    return waveAddressFromAccountId(this.accountId);
  }

  get waveAccount(): string {
    return deriveAccount(this.accountId, "WAVE");
  }

  get usdcAccount(): string {
    return deriveAccount(this.accountId, "USDC");
  }

  sign(message: Uint8Array): Uint8Array {
    return ed25519.sign(message, this.secretKey);
  }

  verify(message: Uint8Array, signature: Uint8Array): boolean {
    return ed25519.verify(signature, message, this.publicKey);
  }

  toJSON() {
    return {
      publicKey: bytesToHex(this.publicKey),
      accountId: this.accountId,
      waveAddress: this.waveAddress,
      waveAccount: this.waveAccount,
      usdcAccount: this.usdcAccount,
    };
  }
}
