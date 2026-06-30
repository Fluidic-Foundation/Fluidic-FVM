"use client";

import Link from "next/link";
import dynamic from "next/dynamic";
import { useEffect, useState } from "react";
import {
  ArrowLeft,
  Beaker,
  Copy,
  FlaskConical,
  Wallet,
  Droplets,
  ArrowRightLeft,
  Terminal,
  Check,
  AlertCircle,
  RefreshCw,
} from "lucide-react";

const API_URL = "https://api.testnet.fluidic.foundation";

const CodeRunner = dynamic(() => import("./code-runner").then((m) => m.CodeRunner), {
  ssr: false,
});

type FluidicSdk = typeof import("@fluidic-foundation/sdk");

interface AccountInfo {
  accountId: string;
  publicKey: string;
  secretKey: string;
  waveAccount: string;
  usdcAccount: string;
}

function formatUnits(raw: string, decimals = 12): string {
  try {
    const n = BigInt(raw);
    const divisor = BigInt(10 ** decimals);
    const intPart = n / divisor;
    const fracPart = n % divisor;
    const frac = fracPart.toString().padStart(decimals, "0").replace(/0+$/, "");
    return frac ? `${intPart}.${frac}` : intPart.toString();
  } catch {
    return raw;
  }
}

export default function PlaygroundPage() {
  const [sdk, setSdk] = useState<FluidicSdk | null>(null);
  const [kp, setKp] = useState<AccountInfo | null>(null);
  const [balances, setBalances] = useState<{ wave: string; usdc: string } | null>(null);
  const [registering, setRegistering] = useState(false);
  const [registerError, setRegisterError] = useState<string | null>(null);
  const [swapStatus, setSwapStatus] = useState<string>("");
  const [swapLoading, setSwapLoading] = useState(false);
  const [direction, setDirection] = useState<"WAVE_TO_USDC" | "USDC_TO_WAVE">("WAVE_TO_USDC");
  const [amount, setAmount] = useState<string>("10");
  const [swapNonces, setSwapNonces] = useState<Record<string, number>>({});
  const [evmAddress, setEvmAddress] = useState<string>("");
  const [faucetStatus, setFaucetStatus] = useState<string>("");
  const [rawMethod, setRawMethod] = useState<"GET" | "POST">("GET");
  const [rawPath, setRawPath] = useState<string>("/api/state");
  const [rawBody, setRawBody] = useState<string>("");
  const [rawResponse, setRawResponse] = useState<string>("");

  useEffect(() => {
    import("@fluidic-foundation/sdk").then((mod) => setSdk(mod));
  }, []);

  useEffect(() => {
    if (!kp) return;
    const tick = () =>
      fetch(`${API_URL}/api/account/${kp.accountId}/balance`)
        .then((r) => r.json())
        .then((data) => setBalances(data))
        .catch(() => null);
    tick();
    const id = setInterval(tick, 3000);
    return () => clearInterval(id);
  }, [kp]);

  const generate = () => {
    if (!sdk) return;
    const key = sdk.FluidicKeypair.generate();
    setKp({
      accountId: key.accountId,
      publicKey: key.publicKeyHex,
      secretKey: key.secretKeyHex,
      waveAccount: key.waveAccount,
      usdcAccount: key.usdcAccount,
    });
    setBalances(null);
    setRegisterError(null);
    setFaucetStatus("");
  };

  const importKey = (hex: string) => {
    if (!sdk) return;
    try {
      const key = sdk.FluidicKeypair.fromSecretKey(hex.trim());
      setKp({
        accountId: key.accountId,
        publicKey: key.publicKeyHex,
        secretKey: key.secretKeyHex,
        waveAccount: key.waveAccount,
        usdcAccount: key.usdcAccount,
      });
      setRegisterError(null);
      setFaucetStatus("");
    } catch (e: any) {
      setRegisterError(e.message || "Invalid secret key");
    }
  };

  const registerAccount = async () => {
    if (!kp) return;
    setRegistering(true);
    setRegisterError(null);
    try {
      const res = await fetch(`${API_URL}/api/account/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ public_key_hex: kp.publicKey }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(JSON.stringify(data));
      setFaucetStatus("Account registered and faucet seeded.");
    } catch (e: any) {
      setRegisterError(e.message);
    } finally {
      setRegistering(false);
    }
  };

  const evmFaucet = async () => {
    setFaucetStatus("Sending...");
    try {
      const res = await fetch(`${API_URL}/api/evm/faucet`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ address: evmAddress }),
      });
      const data = await res.json();
      setFaucetStatus(res.ok ? `Sent: ${JSON.stringify(data)}` : `Error: ${JSON.stringify(data)}`);
    } catch (e: any) {
      setFaucetStatus(`Error: ${e.message}`);
    }
  };

  const submitSwap = async () => {
    if (!sdk || !kp) return;
    setSwapLoading(true);
    setSwapStatus("Building swap...");
    try {
      const client = new sdk.FluidicClient({ apiUrl: API_URL });
      const signer = sdk.FluidicKeypair.fromSecretKey(kp.secretKey);
      const fromAccount = direction === "WAVE_TO_USDC" ? signer.waveAccount : signer.usdcAccount;
      const nonce = (swapNonces[fromAccount] ?? 0) + 1;
      const vectorClock = { entries: { [fromAccount]: BigInt(nonce) } };
      const result = await sdk.submitSwap(client, {
        signer,
        direction,
        amount: BigInt(Math.floor(Number(amount) * 1e12)),
        vectorClock,
      });
      setSwapNonces((prev) => ({ ...prev, [fromAccount]: nonce }));
      setSwapStatus(`Swap queued. Pool-in hash: ${result.poolInHash}`);
    } catch (e: any) {
      setSwapStatus(`Swap failed: ${e.message}`);
    } finally {
      setSwapLoading(false);
    }
  };

  const sendRaw = async () => {
    setRawResponse("Loading...");
    try {
      const opts: RequestInit = { method: rawMethod };
      if (rawMethod === "POST" && rawBody) {
        opts.headers = { "Content-Type": "application/json" };
        opts.body = rawBody;
      }
      const res = await fetch(`${API_URL}${rawPath}`, opts);
      const text = await res.text();
      try {
        setRawResponse(JSON.stringify(JSON.parse(text), null, 2));
      } catch {
        setRawResponse(text);
      }
    } catch (e: any) {
      setRawResponse(`Error: ${e.message}`);
    }
  };

  const copy = (text: string) => navigator.clipboard.writeText(text);

  return (
    <div className="min-h-screen bg-neutral-950 text-slate-100">
      <header className="border-b border-white/10 bg-black/40 backdrop-blur-sm px-6 py-4 flex items-center gap-4 sticky top-0 z-10">
        <Link href="/" className="flex items-center gap-2 text-sky-400 hover:text-white transition">
          <ArrowLeft className="w-5 h-5" />
          <span className="font-mono text-sm font-semibold">FLUIDIC</span>
        </Link>
        <div className="flex items-center gap-2 ml-4">
          <FlaskConical className="w-5 h-5 text-sky-400" />
          <h1 className="text-lg font-semibold tracking-tight">Developer Console</h1>
        </div>
      </header>

      <main className="max-w-6xl mx-auto px-4 sm:px-6 py-6 sm:py-10 space-y-6">
        {/* Hero */}
        <section className="space-y-4">
          <div className="space-y-2">
            <h2 className="text-3xl font-bold tracking-tight">Playground</h2>
            <p className="text-slate-400 max-w-2xl">
              Generate a Fluidic identity, fund it from the testnet faucet, submit signed swaps,
              and experiment with the SDK — no local node required.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <InstallPill />
            <span className="text-xs text-slate-500">Testnet API: {API_URL}</span>
          </div>
        </section>

        {/* Wallet + Swap */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Wallet */}
          <Card>
            <div className="flex items-center justify-between">
              <SectionTitle icon={<Wallet className="w-5 h-5 text-sky-400" />} title="Wallet" />
              <button
                onClick={generate}
                disabled={!sdk}
                className="px-3 py-1.5 rounded-lg bg-sky-500 hover:bg-sky-400 disabled:opacity-50 text-black font-medium text-sm transition"
              >
                Generate keypair
              </button>
            </div>

            <div className="space-y-3">
              <label className="block text-xs font-medium uppercase tracking-wider text-slate-500">
                Import secret key (hex)
              </label>
              <input
                type="text"
                placeholder="0000..."
                onChange={(e) => importKey(e.target.value)}
                className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 font-mono text-sm text-slate-200 focus:outline-none focus:border-sky-500 focus:ring-1 focus:ring-sky-500/30 transition"
              />
            </div>

            {kp ? (
              <div className="space-y-2 rounded-xl bg-neutral-950/60 border border-white/5 p-4">
                <KeyRow label="Account" value={kp.accountId} onCopy={() => copy(kp.accountId)} />
                <KeyRow label="Public key" value={kp.publicKey} onCopy={() => copy(kp.publicKey)} />
                <KeyRow label="Secret key" value={kp.secretKey} onCopy={() => copy(kp.secretKey)} masked />
                <KeyRow label="WAVE account" value={kp.waveAccount} onCopy={() => copy(kp.waveAccount)} />
                <KeyRow label="USDC account" value={kp.usdcAccount} onCopy={() => copy(kp.usdcAccount)} />
              </div>
            ) : (
              <div className="rounded-xl bg-neutral-950/60 border border-white/5 p-6 text-center text-sm text-slate-500">
                No identity yet. Generate or import a keypair to start.
              </div>
            )}

            <div className="pt-2 border-t border-white/5">
              <div className="flex items-center gap-2 mb-3">
                <Droplets className="w-4 h-4 text-emerald-400" />
                <span className="text-sm font-medium text-slate-200">Faucet</span>
              </div>
              <div className="flex flex-wrap gap-3 items-center">
                <button
                  onClick={registerAccount}
                  disabled={!kp || registering}
                  className="px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 disabled:opacity-50 text-black font-medium text-sm transition"
                >
                  {registering ? "Registering..." : "Register + fund account"}
                </button>
                {balances && (
                  <div className="flex gap-3 text-sm">
                    <BalanceBadge label="WAVE" value={formatUnits(balances.wave)} />
                    <BalanceBadge label="USDC" value={formatUnits(balances.usdc)} />
                  </div>
                )}
              </div>
              {registerError && (
                <p className="mt-3 flex items-center gap-1.5 text-rose-400 text-sm">
                  <AlertCircle className="w-4 h-4" /> {registerError}
                </p>
              )}
              {faucetStatus && (
                <p className="mt-3 flex items-center gap-1.5 text-emerald-400 text-sm">
                  <Check className="w-4 h-4" /> {faucetStatus}
                </p>
              )}

              <div className="mt-4 flex flex-col sm:flex-row gap-3">
                <input
                  type="text"
                  placeholder="0x... EVM address"
                  value={evmAddress}
                  onChange={(e) => setEvmAddress(e.target.value)}
                  className="flex-1 bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 font-mono text-sm text-slate-200 focus:outline-none focus:border-sky-500 focus:ring-1 focus:ring-sky-500/30 transition"
                />
                <button
                  onClick={evmFaucet}
                  disabled={!evmAddress}
                  className="px-4 py-2 rounded-lg bg-white/10 hover:bg-white/20 disabled:opacity-50 text-sm font-medium transition"
                >
                  EVM faucet
                </button>
              </div>
            </div>
          </Card>

          {/* Swap */}
          <Card>
            <SectionTitle icon={<ArrowRightLeft className="w-5 h-5 text-violet-400" />} title="Swap" />
            <div className="space-y-3">
              <label className="block text-xs font-medium uppercase tracking-wider text-slate-500">Direction</label>
              <select
                value={direction}
                onChange={(e) => setDirection(e.target.value as any)}
                className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-sm text-slate-200 focus:outline-none focus:border-violet-500 focus:ring-1 focus:ring-violet-500/30 transition"
              >
                <option value="WAVE_TO_USDC">WAVE → USDC</option>
                <option value="USDC_TO_WAVE">USDC → WAVE</option>
              </select>
            </div>
            <div className="space-y-3">
              <label className="block text-xs font-medium uppercase tracking-wider text-slate-500">Amount</label>
              <input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-sm text-slate-200 focus:outline-none focus:border-violet-500 focus:ring-1 focus:ring-violet-500/30 transition"
              />
            </div>
            <button
              onClick={submitSwap}
              disabled={!kp || swapLoading}
              className="w-full px-4 py-2.5 rounded-lg bg-violet-500 hover:bg-violet-400 disabled:opacity-50 text-black font-medium text-sm transition"
            >
              {swapLoading ? "Submitting..." : "Submit swap"}
            </button>
            {swapStatus && (
              <div className="rounded-lg bg-neutral-950/60 border border-white/5 p-3">
                <p className="text-xs uppercase tracking-wider text-slate-500 mb-1">Status</p>
                <p className="text-sm font-mono text-slate-300 break-all">{swapStatus}</p>
              </div>
            )}
          </Card>
        </div>

        {/* API Explorer */}
        <Card>
          <SectionTitle icon={<Terminal className="w-5 h-5 text-amber-400" />} title="API Explorer" />
          <div className="flex flex-col sm:flex-row gap-3">
            <select
              value={rawMethod}
              onChange={(e) => setRawMethod(e.target.value as any)}
              className="bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-sm text-slate-200 focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500/30 transition"
            >
              <option>GET</option>
              <option>POST</option>
            </select>
            <input
              type="text"
              value={rawPath}
              onChange={(e) => setRawPath(e.target.value)}
              className="flex-1 bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 font-mono text-sm text-slate-200 focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500/30 transition"
            />
            <button
              onClick={sendRaw}
              className="px-5 py-2 rounded-lg bg-amber-500 hover:bg-amber-400 text-black font-medium text-sm transition"
            >
              Send
            </button>
          </div>
          {rawMethod === "POST" && (
            <textarea
              value={rawBody}
              onChange={(e) => setRawBody(e.target.value)}
              placeholder='{"address":"0x..."}'
              rows={4}
              className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 font-mono text-sm text-slate-200 focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500/30 transition"
            />
          )}
          {rawResponse && (
            <div className="rounded-xl bg-neutral-950/80 border border-white/5 overflow-hidden">
              <div className="px-4 py-2 border-b border-white/5 flex items-center justify-between">
                <span className="text-xs uppercase tracking-wider text-slate-500">Response</span>
                <button
                  onClick={() => setRawResponse("")}
                  className="text-xs text-slate-500 hover:text-slate-300 transition"
                >
                  Clear
                </button>
              </div>
              <pre className="p-4 font-mono text-xs text-slate-300 overflow-auto max-h-80">{rawResponse}</pre>
            </div>
          )}
        </Card>

        {/* IDE */}
        {sdk && <CodeRunner sdk={sdk} />}
      </main>
    </div>
  );
}

function Card({ children }: { children: React.ReactNode }) {
  return (
    <section className="rounded-2xl border border-white/10 bg-neutral-900/40 backdrop-blur-sm p-6 space-y-5 shadow-sm">
      {children}
    </section>
  );
}

function SectionTitle({ icon, title }: { icon: React.ReactNode; title: string }) {
  return (
    <h3 className="text-lg font-semibold tracking-tight flex items-center gap-2 text-slate-100">
      {icon}
      {title}
    </h3>
  );
}

function BalanceBadge({ label, value }: { label: string; value: string }) {
  return (
    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-white/5 border border-white/10 text-xs">
      <span className="text-slate-500">{label}:</span>
      <span className="font-mono text-slate-200 font-medium">{value}</span>
    </span>
  );
}

function KeyRow({
  label,
  value,
  onCopy,
  masked,
}: {
  label: string;
  value: string;
  onCopy: () => void;
  masked?: boolean;
}) {
  return (
    <div className="flex items-center gap-3 text-sm">
      <span className="w-24 shrink-0 text-slate-500 text-xs uppercase tracking-wider">{label}</span>
      <span className="font-mono text-slate-300 truncate flex-1" title={masked ? undefined : value}>
        {masked ? "•".repeat(64) : value}
      </span>
      <button
        onClick={onCopy}
        className="p-1.5 rounded-md hover:bg-white/10 text-slate-500 hover:text-slate-200 transition"
        title="Copy"
      >
        <Copy className="w-4 h-4" />
      </button>
    </div>
  );
}

function InstallPill() {
  const [copied, setCopied] = useState(false);
  const cmd = "npm install @fluidic-foundation/sdk";
  return (
    <button
      onClick={() => {
        navigator.clipboard.writeText(cmd);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
      className="group inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-neutral-900 border border-white/10 hover:border-sky-500/50 transition"
    >
      <span className="font-mono text-xs text-slate-300">{cmd}</span>
      {copied ? (
        <Check className="w-3.5 h-3.5 text-emerald-400" />
      ) : (
        <Copy className="w-3.5 h-3.5 text-slate-500 group-hover:text-sky-400 transition" />
      )}
    </button>
  );
}
