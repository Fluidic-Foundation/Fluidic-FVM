import {
  FluidicClient as SdkClient,
  FluidicKeypair,
  buildStatefulShift,
  hashStatefulShift,
  type StateSnapshot,
  type ShiftStatusResponse,
  type StateResponse,
} from '@fluidic/sdk';

const API_BASE = import.meta.env.VITE_FLUIDIC_API || 'http://localhost:8080';

export interface FluidicState {
  wave_reserve: string;
  usdc_reserve: string;
  price: number;
  throughput: number;
  latency_ms: number;
  metabolic_burned: string;
  commutative_applied: number;
  stateful_applied: number;
  pool_wave_account: string;
  pool_usdc_account: string;
}

export interface AccountInfo {
  publicKeyHex: string;
  privateKeyHex: string;
  accountId: string;
  waveAccount: string;
  usdcAccount: string;
}

export interface SwapResult {
  status: string;
  hash: string;
}

export interface ShiftStatus {
  hash: string;
  status: 'unknown' | 'accepted' | 'finalized' | 'rejected';
  error: string | null;
  synthesis_tick: number;
  confirmations: number;
}

export interface BalanceResponse {
  wave: string;
  usdc: string;
}

export interface FluidicClient {
  state: FluidicState;
  account: AccountInfo | null;
  isConnected: boolean;
  connect(): void;
  disconnect(): void;
  createAccount(): Promise<AccountInfo>;
  getBalance(accountId: string): Promise<BalanceResponse>;
  swap(from: 'WAVE' | 'USDC', to: 'WAVE' | 'USDC', amount: string): Promise<SwapResult>;
  getShiftStatus(hash: string): Promise<ShiftStatus>;
  subscribe(callback: (state: FluidicState) => void): () => void;
}

function loadOrCreateAccount(): { keypair: FluidicKeypair; info: AccountInfo } {
  const stored = localStorage.getItem('fluidic:account');
  if (stored) {
    const parsed = JSON.parse(stored) as AccountInfo;
    const keypair = FluidicKeypair.fromSecretKey(parsed.privateKeyHex);
    return {
      keypair,
      info: {
        publicKeyHex: keypair.publicKeyHex,
        privateKeyHex: keypair.secretKeyHex,
        accountId: keypair.accountId,
        waveAccount: keypair.waveAccount,
        usdcAccount: keypair.usdcAccount,
      },
    };
  }
  return createNewAccount();
}

function createNewAccount(): { keypair: FluidicKeypair; info: AccountInfo } {
  const keypair = FluidicKeypair.generate();
  const info: AccountInfo = {
    publicKeyHex: keypair.publicKeyHex,
    privateKeyHex: keypair.secretKeyHex,
    accountId: keypair.accountId,
    waveAccount: keypair.waveAccount,
    usdcAccount: keypair.usdcAccount,
  };
  localStorage.setItem('fluidic:account', JSON.stringify(info));
  return { keypair, info };
}

function mapState(snap: StateSnapshot | StateResponse): FluidicState {
  return {
    wave_reserve: snap.wave_reserve,
    usdc_reserve: snap.usdc_reserve,
    price: snap.price,
    throughput: snap.throughput,
    latency_ms: snap.latency_ms,
    metabolic_burned: snap.metabolic_burned,
    commutative_applied: snap.commutative_applied,
    stateful_applied: snap.stateful_applied,
    pool_wave_account: snap.pool_wave_account,
    pool_usdc_account: snap.pool_usdc_account,
  };
}

function mapStatus(status: ShiftStatusResponse): ShiftStatus {
  return {
    hash: status.hash,
    status: status.status,
    error: status.error,
    synthesis_tick: status.synthesis_tick,
    confirmations: status.confirmations,
  };
}

export function createFluidicClient(): FluidicClient {
  const sdk = new SdkClient({
    apiUrl: API_BASE,
    minTick: 'latest',
  });

  let { keypair, info: account } = loadOrCreateAccount();
  let currentState: FluidicState = {
    wave_reserve: '0',
    usdc_reserve: '0',
    price: 0,
    throughput: 0,
    latency_ms: 0,
    metabolic_burned: '0',
    commutative_applied: 0,
    stateful_applied: 0,
    pool_wave_account: '',
    pool_usdc_account: '',
  };
  const listeners = new Set<(state: FluidicState) => void>();
  let registered = false;
  let wsUnsubscribe: (() => void) | null = null;

  const notify = () => listeners.forEach((cb) => cb(currentState));

  const connect = async () => {
    if (wsUnsubscribe) return;

    if (!registered) {
      try {
        const data = await sdk.register(keypair.publicKeyHex);
        account.accountId = data.account_id;
        account.waveAccount = data.wave_account;
        account.usdcAccount = data.usdc_account;
        localStorage.setItem('fluidic:account', JSON.stringify(account));
        registered = true;
      } catch (e) {
        console.error('registration failed', e);
      }
    }

    try {
      currentState = mapState(await sdk.state());
      notify();
    } catch (e) {
      console.error('initial state fetch failed', e);
    }

    wsUnsubscribe = sdk.subscribeSnapshots((snap) => {
      currentState = mapState(snap);
      notify();
    });
  };

  const disconnect = () => {
    wsUnsubscribe?.();
    wsUnsubscribe = null;
  };

  const createAccount = async (): Promise<AccountInfo> => {
    const next = createNewAccount();
    keypair = next.keypair;
    account = next.info;
    registered = false;
    await connect();
    return account;
  };

  const swap = async (
    from: 'WAVE' | 'USDC',
    to: 'WAVE' | 'USDC',
    amount: string
  ): Promise<SwapResult> => {
    if (from === to) throw new Error('cannot swap a token for itself');

    const state = await sdk.state();
    if (!state.pool_wave_account || !state.pool_usdc_account) {
      throw new Error('pool accounts not loaded');
    }

    const toAccount = from === 'WAVE' ? state.pool_wave_account : state.pool_usdc_account;
    const amountBig = BigInt(amount);
    const nonce = BigInt(Date.now());

    const shift = buildStatefulShift({
      signer: keypair,
      to: toAccount,
      amount: amountBig,
      vectorClock: {
        entries: {
          '0000000000000000000000000000000000000000000000000000000000000000': 1n,
        },
      },
      nonce,
      timestampNs: 0n,
    });

    await sdk.submitStateful(shift);
    const hash = hashStatefulShift(shift);

    return { status: 'queued', hash };
  };

  const getShiftStatus = async (hash: string): Promise<ShiftStatus> => {
    return mapStatus(await sdk.shiftStatus(hash));
  };

  const getBalance = async (accountId: string): Promise<BalanceResponse> => {
    return sdk.balance(accountId);
  };

  const subscribe = (callback: (state: FluidicState) => void) => {
    listeners.add(callback);
    callback(currentState);
    return () => listeners.delete(callback);
  };

  return {
    get state() {
      return currentState;
    },
    get account() {
      return account;
    },
    get isConnected() {
      return wsUnsubscribe !== null;
    },
    connect,
    disconnect,
    createAccount,
    getBalance,
    swap,
    getShiftStatus,
    subscribe,
  };
}
