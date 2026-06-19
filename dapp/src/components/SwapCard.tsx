import { useState, useCallback, useMemo, useEffect } from 'react';
import { TOKENS } from '../lib/tokens';
import type { FluidicState, AccountInfo, SwapResult, ShiftStatus } from '../lib/fluidicClient';

interface SwapCardProps {
  state: FluidicState;
  account: AccountInfo | null;
  onSwap: (from: 'WAVE' | 'USDC', to: 'WAVE' | 'USDC', amount: string) => Promise<SwapResult>;
  onGetShiftStatus: (hash: string) => Promise<ShiftStatus>;
  onGetBalance?: (accountId: string) => Promise<{ wave: string; usdc: string }>;
}

export function SwapCard({ state, account, onSwap, onGetShiftStatus, onGetBalance }: SwapCardProps) {
  const [fromSymbol, setFromSymbol] = useState<'WAVE' | 'USDC'>('WAVE');
  const [amount, setAmount] = useState('');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<SwapResult | null>(null);
  const [status, setStatus] = useState<ShiftStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [balance, setBalance] = useState<{ wave: string; usdc: string } | null>(null);

  const toSymbol = fromSymbol === 'WAVE' ? 'USDC' : 'WAVE';

  useEffect(() => {
    if (!account?.accountId || !onGetBalance) return;
    let active = true;
    const fetchBalance = async () => {
      try {
        const b = await onGetBalance(account.accountId);
        if (active) setBalance(b);
      } catch (e) {
        console.error('balance fetch failed', e);
      }
    };
    fetchBalance();
    const id = setInterval(fetchBalance, 3000);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, [account, onGetBalance]);

  useEffect(() => {
    if (!result?.hash) return;

    let active = true;
    const poll = async () => {
      try {
        const s = await onGetShiftStatus(result.hash);
        if (!active) return;
        setStatus(s);
        if (s.status === 'finalized' || s.status === 'rejected') {
          return;
        }
        setTimeout(poll, 2000);
      } catch (e) {
        console.error('status poll failed', e);
      }
    };
    poll();
    return () => { active = false; };
  }, [result?.hash, onGetShiftStatus]);

  const price = state.price || 1;
  const estimated = useMemo(() => {
    const val = parseFloat(amount || '0');
    if (!val || !Number.isFinite(val)) return 0;
    return fromSymbol === 'WAVE' ? val * price : val / price;
  }, [amount, fromSymbol, price]);

  const fromToken = TOKENS.find((t) => t.symbol === fromSymbol)!;
  const fromBalance = balance ? (fromSymbol === 'WAVE' ? balance.wave : balance.usdc) : null;
  const fromBalanceDisplay = useMemo(() => {
    if (!fromBalance) return '—';
    try {
      const base = BigInt(fromBalance);
      const divisor = BigInt(10) ** BigInt(fromToken.decimals);
      const whole = base / divisor;
      const frac = base % divisor;
      const fracStr = frac.toString().padStart(fromToken.decimals, '0').replace(/0+$/, '');
      return fracStr ? `${whole}.${fracStr}` : whole.toString();
    } catch {
      return '—';
    }
  }, [fromBalance, fromToken.decimals]);

  const handleSwap = useCallback(async () => {
    if (!account) return;
    setLoading(true);
    setError(null);
    setResult(null);
    setStatus(null);
    try {
      const baseAmount = toBaseUnits(amount, fromToken.decimals);
      if (!baseAmount) throw new Error('invalid amount');
      const amountBig = BigInt(baseAmount);
      const maxU128 = BigInt('340282366920938463463374607431768211455');
      if (amountBig > maxU128) throw new Error('amount overflows u128');
      if (fromBalance) {
        const bal = BigInt(fromBalance);
        if (amountBig > bal) throw new Error('insufficient balance');
      }
      const res = await onSwap(fromSymbol, toSymbol, baseAmount);
      setResult(res);
      setAmount('');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Swap failed');
    } finally {
      setLoading(false);
    }
  }, [account, amount, fromSymbol, fromBalance, fromToken.decimals, onSwap, toSymbol]);

  if (!account?.accountId) {
    return (
      <div className="bg-fluidic-card border border-fluidic-border rounded-xl p-6 text-center">
        <p className="text-fluidic-muted">Create an account to start swapping.</p>
      </div>
    );
  }

  return (
    <div className="bg-fluidic-card border border-fluidic-border rounded-xl p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold">Swap</h2>
        <span className="text-xs text-fluidic-accent font-mono">NO BLOCK TIME</span>
      </div>

      <TokenInput
        label="From"
        symbol={fromSymbol}
        amount={amount}
        balance={fromBalanceDisplay}
        onAmountChange={setAmount}
      />

      <div className="flex justify-center -my-3 relative z-10">
        <button
          onClick={() => {
            setFromSymbol(toSymbol);
            setAmount(estimated.toFixed(6));
          }}
          className="w-10 h-10 rounded-lg bg-fluidic-bg border border-fluidic-border text-fluidic-accent hover:border-fluidic-accent transition-colors"
        >
          ↓
        </button>
      </div>

      <TokenInput
        label="To (estimated)"
        symbol={toSymbol}
        amount={estimated.toFixed(6)}
        readOnly
      />

      <div className="mt-4 space-y-2 text-sm text-fluidic-muted">
        <div className="flex justify-between">
          <span>Rate</span>
          <span className="text-white font-mono">
            1 WAVE = {price.toFixed(4)} USDC
          </span>
        </div>
        <div className="flex justify-between">
          <span>Network</span>
          <span className="text-white font-mono">Fluidic mesh</span>
        </div>
      </div>

      <button
        onClick={handleSwap}
        disabled={loading || !amount || parseFloat(amount) <= 0}
        className="w-full mt-6 py-3 rounded-lg font-semibold bg-fluidic-accent text-fluidic-bg disabled:opacity-50 hover:opacity-90 transition-opacity"
      >
        {loading ? 'Signing…' : 'Swap Continuously'}
      </button>

      {status && (
        <div className="mt-4 text-center text-sm">
          <span className="text-fluidic-dim">Status: </span>
          <span className={
            status.status === 'finalized'
              ? 'text-green-400 font-mono'
              : status.status === 'rejected'
              ? 'text-red-400 font-mono'
              : 'text-fluidic-accent font-mono animate-pulse'
          }>
            {status.status.toUpperCase()}
            {status.status === 'accepted' && status.confirmations > 0
              ? ` (${status.confirmations} confirmations)`
              : ''}
          </span>
          {status.error && <span className="block text-red-400">{status.error}</span>}
        </div>
      )}
      {!status && result && (
        <p className="mt-4 text-center text-sm text-fluidic-accent">
          Swap queued for synthesis.
        </p>
      )}
      {error && (
        <p className="mt-4 text-center text-sm text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}

interface TokenInputProps {
  label: string;
  symbol: 'WAVE' | 'USDC';
  amount: string;
  balance?: string;
  onAmountChange?: (s: string) => void;
  readOnly?: boolean;
}

function toBaseUnits(amount: string, decimals: number): string | null {
  const trimmed = amount.trim();
  if (!trimmed) return null;
  if (!/^\d*\.?\d+$/.test(trimmed)) return null;
  const [whole = '0', frac = ''] = trimmed.split('.');
  if (frac.length > decimals) return null;
  const padded = frac.padEnd(decimals, '0');
  // Remove leading zeros but keep at least one digit.
  const base = `${whole}${padded}`.replace(/^0+/, '') || '0';
  return base;
}

function TokenInput({ label, symbol, amount, balance, onAmountChange, readOnly }: TokenInputProps) {
  const token = TOKENS.find((t) => t.symbol === symbol)!;
  return (
    <div className="bg-fluidic-bg border border-fluidic-border rounded-xl p-4">
      <div className="flex justify-between text-xs text-fluidic-dim mb-2">
        <span>{label}</span>
        <span className="font-mono">BALANCE: {balance ?? '—'}</span>
      </div>
      <div className="flex items-center gap-3">
        <div
          className="px-3 py-2 rounded-lg text-sm font-semibold text-white border border-fluidic-border"
          style={{ borderColor: token.color }}
        >
          {symbol}
        </div>
        <input
          type="number"
          value={amount}
          onChange={(e) => onAmountChange?.(e.target.value)}
          readOnly={readOnly}
          placeholder="0.0"
          className="flex-1 bg-transparent text-right text-xl font-semibold text-white outline-none placeholder:text-fluidic-dim"
        />
      </div>
    </div>
  );
}
