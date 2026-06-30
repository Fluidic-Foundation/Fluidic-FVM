"use client";

import { useEffect, useState } from "react";
import { FluidicClient, FluidicKeypair, submitSwap } from "@fluidic-foundation/sdk";

const API_URL = "https://api.testnet.fluidic.foundation";

function fmt(raw: string, decimals = 12) {
  const n = BigInt(raw);
  const d = BigInt(10 ** decimals);
  const int = n / d;
  const frac = (n % d).toString().padStart(decimals, "0").replace(/0+$/, "");
  return frac ? `${int}.${frac}` : int.toString();
}

export default function Home() {
  const [signer, setSigner] = useState<FluidicKeypair | null>(null);
  const [balances, setBalances] = useState<{ wave: string; usdc: string } | null>(null);
  const [status, setStatus] = useState<string>("Click 'Generate wallet' to start");
  const [hash, setHash] = useState<string>("");

  const client = new FluidicClient({ apiUrl: API_URL });

  useEffect(() => {
    if (!signer) return;
    const poll = setInterval(() => {
      client.balance(signer.accountId).then(setBalances).catch(() => null);
    }, 3000);
    client.balance(signer.accountId).then(setBalances).catch(() => null);
    return () => clearInterval(poll);
  }, [signer]);

  const generate = () => {
    const s = FluidicKeypair.generate();
    setSigner(s);
    setStatus(`Wallet: ${s.accountId.slice(0, 16)}...`);
  };

  const fund = async () => {
    if (!signer) return;
    setStatus("Funding from testnet faucet...");
    await fetch(`${API_URL}/api/account/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ public_key_hex: signer.publicKeyHex }),
    });
    setStatus("Funded.");
  };

  const swap = async () => {
    if (!signer) return;
    setStatus("Submitting swap...");
    const result = await submitSwap(client, {
      signer,
      direction: "WAVE_TO_USDC",
      amount: 1_000_000_000_000n,
      vectorClock: { entries: { [signer.waveAccount]: 1n } },
    });
    setHash(result.poolInHash);
    setStatus(`Swap submitted: ${result.poolInHash.slice(0, 24)}...`);
  };

  return (
    <main className="min-h-screen p-8 max-w-xl mx-auto space-y-6">
      <h1 className="text-3xl font-bold text-sky-400">Fluidic starter</h1>
      <p className="text-white/60">
        A minimal Next.js app connected to the Fluidic testnet.
      </p>

      <div className="flex gap-3">
        <button onClick={generate} className="px-4 py-2 rounded bg-sky-500 text-black font-semibold">
          1. Generate wallet
        </button>
        <button onClick={fund} disabled={!signer} className="px-4 py-2 rounded bg-emerald-500 text-black font-semibold disabled:opacity-50">
          2. Fund
        </button>
        <button onClick={swap} disabled={!signer} className="px-4 py-2 rounded bg-violet-500 text-black font-semibold disabled:opacity-50">
          3. Swap
        </button>
      </div>

      {signer && (
        <div className="rounded border border-white/10 bg-white/5 p-4 space-y-2 text-sm">
          <p><span className="text-white/40">Account:</span> {signer.accountId}</p>
          <p><span className="text-white/40">Public key:</span> {signer.publicKeyHex.slice(0, 32)}...</p>
          {balances && (
            <p>
              <span className="text-white/40">Balances:</span>{" "}
              {fmt(balances.wave)} WAVE / {fmt(balances.usdc)} USDC
            </p>
          )}
        </div>
      )}

      <div className="rounded border border-white/10 bg-black/40 p-4 text-sm font-mono text-white/70">
        {status}
      </div>

      {hash && (
        <a
          href={`https://testnet.fluidic.foundation/explorer?shift=${hash}`}
          target="_blank"
          rel="noreferrer"
          className="text-sky-400 hover:underline text-sm"
        >
          View shift in explorer →
        </a>
      )}
    </main>
  );
}
