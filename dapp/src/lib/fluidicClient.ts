import {
  FluidicClient as SdkClient,
  FluidicKeypair,
  submitSwap,
  quoteSwap,
  type StateResponse,
} from '@fluidic/sdk';

const API_BASE = import.meta.env.VITE_FLUIDIC_API || 'https://api.testnet.fluidic.foundation';

export interface TokenBalances {
  wave: string;
  usdc: string;
}

export interface Wallet {
  keypair: FluidicKeypair;
  accountId: string;
  waveAccount: string;
  usdcAccount: string;
}

export interface SwapResult {
  poolInHash: string;
  estimatedOut: string;
}

export interface RecentShift {
  hash: string;
  kind: string;
  status: string;
  domain?: string;
  from?: string;
  to?: string;
  amount?: string;
  timestamp_ns: number;
}

const STORAGE_KEY = 'fluidic:dev-wallet';
const VC_KEY = 'fluidic:dev-vc';

function saveWallet(wallet: Wallet) {
  localStorage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      privateKeyHex: wallet.keypair.secretKeyHex,
      accountId: wallet.accountId,
      waveAccount: wallet.waveAccount,
      usdcAccount: wallet.usdcAccount,
    })
  );
}

export function loadWallet(): Wallet | null {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  try {
    const stored = JSON.parse(raw);
    const keypair = FluidicKeypair.fromSecretKey(stored.privateKeyHex);
    return {
      keypair,
      accountId: stored.accountId || keypair.accountId,
      waveAccount: stored.waveAccount || keypair.waveAccount,
      usdcAccount: stored.usdcAccount || keypair.usdcAccount,
    };
  } catch {
    localStorage.removeItem(STORAGE_KEY);
    return null;
  }
}

export function clearWallet() {
  localStorage.removeItem(STORAGE_KEY);
}

export function createWallet(): Wallet {
  const keypair = FluidicKeypair.generate();
  const wallet: Wallet = {
    keypair,
    accountId: keypair.accountId,
    waveAccount: keypair.waveAccount,
    usdcAccount: keypair.usdcAccount,
  };
  saveWallet(wallet);
  return wallet;
}

export function createClient() {
  return new SdkClient({
    apiUrl: API_BASE,
    minTick: 'none',
  });
}

export async function registerWallet(client: SdkClient, wallet: Wallet) {
  const res = await client.register(wallet.keypair.publicKeyHex);
  return {
    accountId: res.account_id,
    waveAccount: res.wave_account,
    usdcAccount: res.usdc_account,
  };
}

export async function fetchBalances(
  client: SdkClient,
  accountId: string
): Promise<TokenBalances> {
  const res = await client.balance(accountId);
  return {
    wave: res.wave,
    usdc: res.usdc,
  };
}

function vcKey(accountId: string) {
  return `${VC_KEY}:${accountId}`;
}

export function getNextVectorClock(accountId: string): bigint {
  const raw = localStorage.getItem(vcKey(accountId));
  const next = raw ? Number(raw) : 1;
  return BigInt(Math.max(1, Number.isFinite(next) ? next : 1));
}

export function bumpVectorClock(accountId: string) {
  const next = Number(getNextVectorClock(accountId)) + 1;
  localStorage.setItem(vcKey(accountId), next.toString());
}

export async function syncVectorClocks(
  client: SdkClient,
  wallet: Wallet
): Promise<void> {
  try {
    const res = await fetch(`${client.apiUrl}/api/shifts/recent`);
    if (!res.ok) return;
    const shifts = (await res.json()) as RecentShift[];
    const waveCount = shifts.filter(
      (s) => s.from === wallet.waveAccount && s.kind === 'stateful'
    ).length;
    const usdcCount = shifts.filter(
      (s) => s.from === wallet.usdcAccount && s.kind === 'stateful'
    ).length;
    localStorage.setItem(vcKey(wallet.waveAccount), (waveCount + 1).toString());
    localStorage.setItem(vcKey(wallet.usdcAccount), (usdcCount + 1).toString());
  } catch (e) {
    console.error('failed to sync vector clocks', e);
  }
}

export async function executeSwap(
  client: SdkClient,
  wallet: Wallet,
  direction: 'WAVE_TO_USDC' | 'USDC_TO_WAVE',
  amount: bigint
): Promise<SwapResult> {
  const fromAccount = direction === 'WAVE_TO_USDC' ? wallet.waveAccount : wallet.usdcAccount;
  const ownTime = getNextVectorClock(fromAccount);

  const { poolInHash } = await submitSwap(client, {
    signer: wallet.keypair,
    direction,
    amount,
    vectorClock: {
      entries: { [fromAccount]: ownTime },
    },
    nonce: BigInt(Date.now()),
  });

  bumpVectorClock(fromAccount);

  const state = await client.state();
  const waveReserve = BigInt(state.wave_reserve);
  const usdcReserve = BigInt(state.usdc_reserve);
  const quote = quoteSwap(direction, amount, waveReserve, usdcReserve);

  return {
    poolInHash,
    estimatedOut: quote.amountOut.toString(),
  };
}

export function subscribeToState(
  client: SdkClient,
  callback: (state: StateResponse) => void
): () => void {
  return client.subscribeSnapshots((snap) => {
    callback(snap as StateResponse);
  });
}
