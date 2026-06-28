"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { ArrowLeft, Beaker, Copy, FlaskConical, RefreshCw } from "lucide-react";

const API_URL = "https://api.testnet.fluidic.foundation";

type FluidicSdk = typeof import("@fluidic/sdk");

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
    import("@fluidic/sdk").then((mod) => setSdk(mod));
  }, []);

  useEffect(() => {
    if (!kp) return;
    const id = setInterval(() => {
      fetch(`${API_URL}/api/account/${kp.accountId}/balance`)
        .then((r) => r.json())
        .then((data) => setBalances(data))
        .catch(() => null);
    }, 3000);
    fetch(`${API_URL}/api/account/${kp.accountId}/balance`)
      .then((r) => r.json())
      .then((data) => setBalances(data))
      .catch(() => null);
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
    <div className="min-h-screen bg-black text-white">
      <header className="border-b border-white/10 px-6 py-4 flex items-center gap-4">
        <Link href="/" className="flex items-center gap-2 text-primary hover:text-white transition">
          <ArrowLeft className="w-5 h-5" />
          <span className="font-mono text-sm">FLUIDIC</span>
        </Link>
        <div className="flex items-center gap-2 ml-4">
          <FlaskConical className="w-5 h-5 text-sky-400" />
          <h1 className="text-lg font-semibold tracking-tight">Developer Console</h1>
        </div>
      </header>

      <main className="max-w-6xl mx-auto px-6 py-10 space-y-8">
        <div className="space-y-2">
          <h2 className="text-3xl font-bold">Playground</h2>
          <p className="text-white/60 max-w-2xl">
            Generate a Fluidic identity, fund it from the testnet faucet, submit
            signed swaps, and experiment with the REST API — no local node required.
          </p>
        </div>

        {/* Identity */}
        <section className="rounded-2xl border border-white/10 bg-white/5 p-6 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-xl font-semibold flex items-center gap-2">
              <Beaker className="w-5 h-5 text-sky-400" /> Identity
            </h3>
            <div className="flex gap-2">
              <button
                onClick={generate}
                disabled={!sdk}
                className="px-4 py-2 rounded-lg bg-sky-500 hover:bg-sky-400 disabled:opacity-50 text-black font-medium text-sm transition"
              >
                Generate keypair
              </button>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-xs uppercase tracking-wider text-white/40">Or import secret key (hex)</label>
              <input
                type="text"
                placeholder="0000..."
                onChange={(e) => importKey(e.target.value)}
                className="w-full bg-black border border-white/20 rounded-lg px-3 py-2 font-mono text-sm focus:outline-none focus:border-sky-500"
              />
            </div>
            <div className="space-y-2">
              <label className="text-xs uppercase tracking-wider text-white/40">API endpoint</label>
              <div className="w-full bg-black border border-white/20 rounded-lg px-3 py-2 font-mono text-sm text-white/60">
                {API_URL}
              </div>
            </div>
          </div>

          {kp && (
            <div className="space-y-3 rounded-xl bg-black/40 p-4 border border-white/10">
              <KeyRow label="Account" value={kp.accountId} onCopy={() => copy(kp.accountId)} />
              <KeyRow label="Public key" value={kp.publicKey} onCopy={() => copy(kp.publicKey)} />
              <KeyRow label="Secret key" value={kp.secretKey} onCopy={() => copy(kp.secretKey)} masked />
              <KeyRow label="WAVE account" value={kp.waveAccount} onCopy={() => copy(kp.waveAccount)} />
              <KeyRow label="USDC account" value={kp.usdcAccount} onCopy={() => copy(kp.usdcAccount)} />
            </div>
          )}
        </section>

        {/* Faucet */}
        <section className="rounded-2xl border border-white/10 bg-white/5 p-6 space-y-4">
          <h3 className="text-xl font-semibold">Faucet</h3>
          <div className="flex flex-wrap gap-3 items-center">
            <button
              onClick={registerAccount}
              disabled={!kp || registering}
              className="px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 disabled:opacity-50 text-black font-medium text-sm transition"
            >
              {registering ? "Registering..." : "Register Fluidic account + faucet"}
            </button>
            {balances && (
              <div className="flex gap-4 text-sm">
                <span className="text-white/60">
                  WAVE: <strong className="text-white">{formatUnits(balances.wave)}</strong>
                </span>
                <span className="text-white/60">
                  USDC: <strong className="text-white">{formatUnits(balances.usdc)}</strong>
                </span>
              </div>
            )}
          </div>
          {registerError && <p className="text-rose-400 text-sm">{registerError}</p>}
          {faucetStatus && <p className="text-emerald-400 text-sm">{faucetStatus}</p>}

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3 pt-2">
            <input
              type="text"
              placeholder="0x... EVM address"
              value={evmAddress}
              onChange={(e) => setEvmAddress(e.target.value)}
              className="md:col-span-2 bg-black border border-white/20 rounded-lg px-3 py-2 font-mono text-sm focus:outline-none focus:border-sky-500"
            />
            <button
              onClick={evmFaucet}
              disabled={!evmAddress}
              className="px-4 py-2 rounded-lg bg-white/10 hover:bg-white/20 disabled:opacity-50 text-sm font-medium transition"
            >
              EVM faucet
            </button>
          </div>
        </section>

        {/* Swap */}
        <section className="rounded-2xl border border-white/10 bg-white/5 p-6 space-y-4">
          <h3 className="text-xl font-semibold">Swap</h3>
          <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
            <select
              value={direction}
              onChange={(e) => setDirection(e.target.value as any)}
              className="bg-black border border-white/20 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-sky-500"
            >
              <option value="WAVE_TO_USDC">WAVE → USDC</option>
              <option value="USDC_TO_WAVE">USDC → WAVE</option>
            </select>
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              className="md:col-span-2 bg-black border border-white/20 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-sky-500"
            />
            <button
              onClick={submitSwap}
              disabled={!kp || swapLoading}
              className="px-4 py-2 rounded-lg bg-violet-500 hover:bg-violet-400 disabled:opacity-50 text-black font-medium text-sm transition"
            >
              {swapLoading ? "Submitting..." : "Submit swap"}
            </button>
          </div>
          {swapStatus && <p className="text-sm text-white/70 font-mono">{swapStatus}</p>}
        </section>

        {/* API Explorer */}
        <section className="rounded-2xl border border-white/10 bg-white/5 p-6 space-y-4">
          <h3 className="text-xl font-semibold">API Explorer</h3>
          <div className="flex gap-3">
            <select
              value={rawMethod}
              onChange={(e) => setRawMethod(e.target.value as any)}
              className="bg-black border border-white/20 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-sky-500"
            >
              <option>GET</option>
              <option>POST</option>
            </select>
            <input
              type="text"
              value={rawPath}
              onChange={(e) => setRawPath(e.target.value)}
              className="flex-1 bg-black border border-white/20 rounded-lg px-3 py-2 font-mono text-sm focus:outline-none focus:border-sky-500"
            />
            <button
              onClick={sendRaw}
              className="px-4 py-2 rounded-lg bg-sky-500 hover:bg-sky-400 text-black font-medium text-sm transition"
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
              className="w-full bg-black border border-white/20 rounded-lg px-3 py-2 font-mono text-sm focus:outline-none focus:border-sky-500"
            />
          )}
          {rawResponse && (
            <pre className="rounded-xl bg-black/60 p-4 border border-white/10 font-mono text-xs overflow-auto max-h-80">
              {rawResponse}
            </pre>
          )}
        </section>

        {/* Code snippets */}
        <section className="rounded-2xl border border-white/10 bg-white/5 p-6 space-y-4">
          <h3 className="text-xl font-semibold">Quick code</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <CodeCard title="Install SDK" lang="bash" code="npm install @fluidic/sdk" />
            <CodeCard
              title="Register + faucet"
              lang="js"
              code={`import { FluidicClient, FluidicKeypair, submitSwap } from "@fluidic/sdk";

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const signer = FluidicKeypair.generate();

await fetch("${API_URL}/api/account/register", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ public_key_hex: signer.publicKeyHex }),
});

const result = await submitSwap(client, {
  signer,
  direction: "WAVE_TO_USDC",
  amount: 1000000000000n, // 1 WAVE
  vectorClock: { entries: { [signer.waveAccount]: 1n } },
});
console.log(result);`}
            />
          </div>
        </section>
      </main>
    </div>
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
      <span className="w-28 shrink-0 text-white/40 text-xs uppercase tracking-wider">{label}</span>
      <span className="font-mono text-white/80 truncate flex-1">{masked ? "•".repeat(64) : value}</span>
      <button
        onClick={onCopy}
        className="p-1.5 rounded-md hover:bg-white/10 text-white/50 hover:text-white transition"
        title="Copy"
      >
        <Copy className="w-4 h-4" />
      </button>
    </div>
  );
}

function CodeCard({ title, lang, code }: { title: string; lang: string; code: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="rounded-xl border border-white/10 bg-black/40 p-4 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-white/80">{title}</span>
        <button
          onClick={() => {
            navigator.clipboard.writeText(code);
            setCopied(true);
            setTimeout(() => setCopied(false), 1500);
          }}
          className="text-xs text-white/40 hover:text-white transition"
        >
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre className="font-mono text-xs text-white/70 overflow-auto max-h-48">
        <code>{code}</code>
      </pre>
    </div>
  );
}
