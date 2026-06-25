import { useEffect, useMemo, useState } from 'react';
import {
  clearWallet,
  createClient,
  createWallet,
  executeSwap,
  fetchBalances,
  loadWallet,
  registerWallet,
  subscribeToState,
  syncVectorClocks,
  type Wallet,
  type TokenBalances,
} from './lib/fluidicClient';
import { TOKENS } from './lib/tokens';

const DECIMALS = 12;
const FORMAT = new Intl.NumberFormat('en-US', { maximumFractionDigits: 4 });

function formatBase(amount: string | bigint): string {
  try {
    const value = BigInt(amount);
    const divisor = BigInt(10) ** BigInt(DECIMALS);
    const whole = value / divisor;
    const frac = value % divisor;
    const fracStr = frac.toString().padStart(DECIMALS, '0').replace(/0+$/, '');
    const num = fracStr ? Number(`${whole}.${fracStr}`) : Number(whole);
    return FORMAT.format(num);
  } catch {
    return '0';
  }
}

function toBaseUnits(input: string): bigint | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  if (!/^\d*\.?\d+$/.test(trimmed)) return null;
  const [whole = '0', frac = ''] = trimmed.split('.');
  if (frac.length > DECIMALS) return null;
  const padded = frac.padEnd(DECIMALS, '0');
  return BigInt(`${whole}${padded}`.replace(/^0+/, '') || '0');
}

export default function App() {
  const client = useMemo(() => createClient(), []);
  const [wallet, setWallet] = useState<Wallet | null>(loadWallet);
  const [balances, setBalances] = useState<TokenBalances>({ wave: '0', usdc: '0' });
  const [connected, setConnected] = useState(false);
  const [statePrice, setStatePrice] = useState(1);

  useEffect(() => {
    const unsub = subscribeToState(client, (state) => {
      setConnected(true);
      setStatePrice(Number(state.price) || 1);
    });
    const health = setInterval(() => {
      client.state().then((s) => setStatePrice(Number(s.price) || 1)).catch(() => setConnected(false));
    }, 5000);
    return () => {
      unsub();
      clearInterval(health);
    };
  }, [client]);

  useEffect(() => {
    if (!wallet) return;
    syncVectorClocks(client, wallet);
    let active = true;
    const poll = async () => {
      try {
        const b = await fetchBalances(client, wallet.accountId);
        if (active) setBalances(b);
      } catch (e) {
        console.error('balance poll failed', e);
      }
    };
    poll();
    const id = setInterval(poll, 3000);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, [client, wallet]);

  const handleCreate = async () => {
    const newWallet = createWallet();
    await registerWallet(client, newWallet);
    await syncVectorClocks(client, newWallet);
    setWallet(newWallet);
  };

  const handleReset = () => {
    clearWallet();
    setWallet(null);
    setBalances({ wave: '0', usdc: '0' });
  };

  return (
    <div className="min-h-screen bg-fluidic-bg text-white flex flex-col">
      <header className="border-b border-fluidic-border bg-fluidic-bg/80 backdrop-blur">
        <div className="max-w-3xl mx-auto px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <img src="/fluidic-logo-new.png" alt="Fluidic" className="h-8 w-8 object-contain" />
            <div>
              <h1 className="text-lg font-bold">Fluidic</h1>
              <p className="text-[10px] text-fluidic-dim font-mono uppercase tracking-widest">Dev Reference DApp</p>
            </div>
          </div>
          <div className="flex items-center gap-2 text-xs font-mono">
            <span className={`w-2 h-2 rounded-full ${connected ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-fluidic-muted">{connected ? 'TESTNET ONLINE' : 'OFFLINE'}</span>
          </div>
        </div>
      </header>

      <main className="flex-1 max-w-3xl w-full mx-auto px-6 py-10 space-y-8">
        {!wallet ? (
          <WelcomeCard onCreate={handleCreate} />
        ) : (
          <>
            <WalletCard wallet={wallet} onReset={handleReset} />
            <FaucetCard client={client} wallet={wallet} onFund={() => fetchBalances(client, wallet.accountId).then(setBalances)} />
            <SwapCard client={client} wallet={wallet} balances={balances} price={statePrice} />
            <BalanceCard balances={balances} />
          </>
        )}
      </main>

      <footer className="py-6 text-center text-[10px] text-fluidic-dim font-mono">
        FLUIDIC CONTINUOUS-STATE DAPP — CONNECTED TO TESTNET MESH
      </footer>
    </div>
  );
}

function WelcomeCard({ onCreate }: { onCreate: () => void }) {
  return (
    <div className="bg-fluidic-card border border-fluidic-border rounded-2xl p-8 text-center space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-bold">Build on Fluidic</h2>
        <p className="text-fluidic-muted text-sm max-w-md mx-auto">
          This reference dApp shows the three core interactions: create a wallet, receive test tokens from the faucet, and swap WAVE/USDC on the continuous-state mesh.
        </p>
      </div>
      <button
        onClick={onCreate}
        className="px-6 py-3 rounded-lg font-semibold bg-fluidic-accent text-fluidic-bg hover:opacity-90 transition-opacity"
      >
        1. Create Wallet
      </button>
      <p className="text-xs text-fluidic-dim">A new Ed25519 keypair is generated in your browser and saved locally.</p>
    </div>
  );
}

function WalletCard({ wallet, onReset }: { wallet: Wallet; onReset: () => void }) {
  const [copied, setCopied] = useState(false);

  const copy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="bg-fluidic-card border border-fluidic-border rounded-2xl p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold uppercase tracking-wider text-fluidic-muted">Wallet</h3>
        <button onClick={onReset} className="text-xs text-red-400 hover:text-red-300">Reset</button>
      </div>
      <div className="space-y-2">
        <div className="flex items-center justify-between bg-fluidic-bg border border-fluidic-border rounded-lg px-4 py-3">
          <span className="text-xs text-fluidic-dim font-mono">ACCOUNT</span>
          <div className="flex items-center gap-2">
            <span className="font-mono text-sm">{wallet.accountId.slice(0, 12)}…{wallet.accountId.slice(-8)}</span>
            <button onClick={() => copy(wallet.accountId)} className="text-xs text-fluidic-accent hover:underline">
              {copied ? 'copied' : 'copy'}
            </button>
          </div>
        </div>
        <div className="flex items-center justify-between bg-fluidic-bg border border-fluidic-border rounded-lg px-4 py-3">
          <span className="text-xs text-fluidic-dim font-mono">PUBLIC KEY</span>
          <span className="font-mono text-sm">{wallet.keypair.publicKeyHex.slice(0, 16)}…</span>
        </div>
      </div>
    </div>
  );
}

function BalanceCard({ balances }: { balances: TokenBalances }) {
  return (
    <div className="grid grid-cols-2 gap-4">
      <div className="bg-fluidic-card border border-fluidic-border rounded-2xl p-5">
        <div className="text-xs text-fluidic-dim font-mono uppercase">WAVE Balance</div>
        <div className="mt-2 text-2xl font-bold text-fluidic-accent">{formatBase(balances.wave)} <span className="text-sm font-normal text-fluidic-muted">WAVE</span></div>
      </div>
      <div className="bg-fluidic-card border border-fluidic-border rounded-2xl p-5">
        <div className="text-xs text-fluidic-dim font-mono uppercase">USDC Balance</div>
        <div className="mt-2 text-2xl font-bold text-blue-400">{formatBase(balances.usdc)} <span className="text-sm font-normal text-fluidic-muted">USDC</span></div>
      </div>
    </div>
  );
}

function FaucetCard({
  client,
  wallet,
  onFund,
}: {
  client: ReturnType<typeof createClient>;
  wallet: Wallet;
  onFund: () => void;
}) {
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [cooldown, setCooldown] = useState(0);

  const request = async () => {
    setLoading(true);
    setMessage(null);
    try {
      await registerWallet(client, wallet);
      await onFund();
      setMessage('Dripped 1,000 WAVE + 1,000 USDC.');
      setCooldown(60);
    } catch (e) {
      setMessage(e instanceof Error ? e.message : 'Faucet request failed');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (cooldown <= 0) return;
    const id = setInterval(() => setCooldown((c) => c - 1), 1000);
    return () => clearInterval(id);
  }, [cooldown]);

  return (
    <div className="bg-fluidic-card border border-fluidic-border rounded-2xl p-6 space-y-4">
      <h3 className="text-sm font-semibold uppercase tracking-wider text-fluidic-muted">2. Faucet</h3>
      <p className="text-sm text-fluidic-muted">
        Your wallet is funded automatically on creation. Request another drip if you run low.
      </p>
      <button
        onClick={request}
        disabled={loading || cooldown > 0}
        className="px-5 py-2.5 rounded-lg font-semibold bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
      >
        {loading ? 'Dripping…' : cooldown > 0 ? `Wait ${cooldown}s` : 'Get 1,000 WAVE + 1,000 USDC'}
      </button>
      {message && <p className={`text-xs ${message.includes('failed') || message.includes('error') ? 'text-red-400' : 'text-green-400'}`}>{message}</p>}
    </div>
  );
}

function SwapCard({
  client,
  wallet,
  balances,
  price,
}: {
  client: ReturnType<typeof createClient>;
  wallet: Wallet;
  balances: TokenBalances;
  price: number;
}) {
  const [direction, setDirection] = useState<'WAVE_TO_USDC' | 'USDC_TO_WAVE'>('WAVE_TO_USDC');
  const [amount, setAmount] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [submittedAt, setSubmittedAt] = useState<number | null>(null);
  const [elapsedMs, setElapsedMs] = useState(0);

  useEffect(() => {
    if (!submittedAt) return;
    if (status === 'finalized' || status === 'rejected') {
      setElapsedMs(Date.now() - submittedAt);
      return;
    }
    const id = setInterval(() => setElapsedMs(Date.now() - submittedAt), 100);
    return () => clearInterval(id);
  }, [submittedAt, status]);

  const fromSymbol = direction === 'WAVE_TO_USDC' ? 'WAVE' : 'USDC';
  const toSymbol = fromSymbol === 'WAVE' ? 'USDC' : 'WAVE';

  const estimated = useMemo(() => {
    const base = toBaseUnits(amount);
    if (!base) return '0';
    const waveReserve = BigInt(balances.wave);
    const usdcReserve = BigInt(balances.usdc);
    if (waveReserve === 0n || usdcReserve === 0n) return formatBase('0');
    const out = direction === 'WAVE_TO_USDC'
      ? (base * usdcReserve) / waveReserve
      : (base * waveReserve) / usdcReserve;
    return formatBase(out.toString());
  }, [amount, direction, balances]);

  const handleSwap = async () => {
    setError(null);
    setStatus(null);
    const base = toBaseUnits(amount);
    if (!base) {
      setError('Invalid amount');
      return;
    }
    const balance = fromSymbol === 'WAVE' ? balances.wave : balances.usdc;
    if (base > BigInt(balance)) {
      setError(`Insufficient ${fromSymbol} balance`);
      return;
    }
    setLoading(true);
    setSubmittedAt(Date.now());
    try {
      const res = await executeSwap(client, wallet, direction, base);
      pollStatus(res.poolInHash);
      setAmount('');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Swap failed');
      setSubmittedAt(null);
      setElapsedMs(0);
    } finally {
      setLoading(false);
    }
  };

  const pollStatus = (hash: string) => {
    setStatus('queued');
    const run = async () => {
      try {
        const s = await client.shiftStatus(hash);
        setStatus(s.status);
        if (s.status !== 'finalized' && s.status !== 'rejected') {
          setTimeout(run, 1500);
        }
      } catch (e) {
        console.error('status poll failed', e);
      }
    };
    run();
  };

  const fromToken = TOKENS.find((t) => t.symbol === fromSymbol)!;
  const fromBalance = fromSymbol === 'WAVE' ? balances.wave : balances.usdc;

  return (
    <div className="bg-fluidic-card border border-fluidic-border rounded-2xl p-6 space-y-5">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold uppercase tracking-wider text-fluidic-muted">3. Swap</h3>
        <span className="text-xs font-mono text-fluidic-accent">1 WAVE ≈ {price.toFixed(4)} USDC</span>
      </div>

      <div className="bg-fluidic-bg border border-fluidic-border rounded-xl p-4 space-y-2">
        <div className="flex justify-between text-xs text-fluidic-dim">
          <span>From</span>
          <span className="font-mono">BALANCE: {formatBase(fromBalance)} {fromSymbol}</span>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setDirection((d) => (d === 'WAVE_TO_USDC' ? 'USDC_TO_WAVE' : 'WAVE_TO_USDC'))}
            className="px-3 py-2 rounded-lg text-sm font-semibold border border-fluidic-border hover:border-fluidic-accent transition-colors"
            style={{ color: fromToken.color, borderColor: fromToken.color }}
          >
            {fromSymbol}
          </button>
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.0"
            className="flex-1 bg-transparent text-right text-xl font-semibold outline-none placeholder:text-fluidic-dim"
          />
        </div>
      </div>

      <div className="flex justify-center -my-2">
        <button
          onClick={() => {
            setDirection(direction === 'WAVE_TO_USDC' ? 'USDC_TO_WAVE' : 'WAVE_TO_USDC');
            setAmount('');
          }}
          className="w-10 h-10 rounded-full bg-fluidic-bg border border-fluidic-border text-fluidic-accent hover:border-fluidic-accent transition-colors"
        >
          ↓
        </button>
      </div>

      <div className="bg-fluidic-bg border border-fluidic-border rounded-xl p-4 space-y-2">
        <div className="flex justify-between text-xs text-fluidic-dim">
          <span>To (estimated)</span>
          <span className="font-mono">{toSymbol}</span>
        </div>
        <div className="text-right text-xl font-semibold text-fluidic-dim">
          {estimated} {toSymbol}
        </div>
      </div>

      <button
        onClick={handleSwap}
        disabled={loading || !amount || Number(amount) <= 0}
        className="w-full py-3 rounded-lg font-semibold bg-fluidic-accent text-fluidic-bg disabled:opacity-50 hover:opacity-90 transition-opacity"
      >
        {loading ? 'Synthesizing…' : 'Swap Continuously'}
      </button>

      {status && (
        <div className="text-center text-xs font-mono">
          <span className="text-fluidic-dim">Status: </span>
          <span
            className={
              status === 'finalized'
                ? 'text-green-400'
                : status === 'rejected'
                ? 'text-red-400'
                : 'text-fluidic-accent animate-pulse'
            }
          >
            {status.toUpperCase()}
          </span>
          {submittedAt && (
            <span className="text-fluidic-dim ml-2">· {(elapsedMs / 1000).toFixed(1)}s</span>
          )}
        </div>
      )}
      {error && <p className="text-center text-xs text-red-400">{error}</p>}
    </div>
  );
}
