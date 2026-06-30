"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { ArrowLeft, Coins, Layers, Loader2, Shield, Wallet } from "lucide-react";
import {
  API_BASE,
  Crumb,
  ExplorerShell,
  KV,
  RecentShift,
  ShiftTable,
  domainHref,
  formatToken,
  isHexAccount,
  shortHash,
} from "../lib";

interface AccountOverview {
  account: string;
  registered: boolean;
  wave_account: string;
  usdc_account: string;
  wave: string;
  usdc: string;
  stake: string;
  is_staked: boolean;
  rewards: string;
  shift_count: number;
  shifts: RecentShift[];
}

function StatCard({
  label,
  value,
  unit,
  icon: Icon,
  accent,
}: {
  label: string;
  value: string;
  unit?: string;
  icon: any;
  accent: string;
}) {
  return (
    <div className="rounded-xl border border-white/[0.06] bg-[#11161c] p-4">
      <div className="mb-2 flex items-center gap-2 text-[10px] uppercase tracking-wider text-slate-500">
        <Icon className="h-3.5 w-3.5" style={{ color: accent }} /> {label}
      </div>
      <div className="font-mono text-xl font-semibold text-white">
        {value}
        {unit && <span className="ml-1.5 text-xs font-normal text-slate-500">{unit}</span>}
      </div>
    </div>
  );
}

function AccountView() {
  const params = useSearchParams();
  const id = (params.get("id") || "").trim();
  const [data, setData] = useState<AccountOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) {
      setError("No account id supplied.");
      setLoading(false);
      return;
    }
    if (!isHexAccount(id)) {
      setError("Invalid account id — expected 64 hex characters.");
      setLoading(false);
      return;
    }
    let active = true;
    const load = async () => {
      try {
        const res = await fetch(`${API_BASE}/api/account/${id}`);
        if (!res.ok) throw new Error("api");
        const json = await res.json();
        if (active) {
          setData(json);
          setError(null);
        }
      } catch {
        if (active) setError("Could not load this account from the mesh.");
      } finally {
        if (active) setLoading(false);
      }
    };
    load();
    const t = setInterval(load, 6_000);
    return () => {
      active = false;
      clearInterval(t);
    };
  }, [id]);

  return (
    <>
      <Crumb label="Wallet" />
      <div className="mb-6 flex items-start gap-4">
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#00d1a7]/10">
          <Wallet className="h-6 w-6 text-[#00d1a7]" />
        </div>
        <div className="min-w-0">
          <h1 className="text-lg font-semibold text-white">Wallet</h1>
          <div className="mt-0.5 break-all font-mono text-xs text-slate-400">{id || "—"}</div>
          {data?.is_staked && (
            <span className="mt-2 inline-flex items-center gap-1 rounded bg-[#8b5cf6]/10 px-2 py-0.5 text-[10px] font-semibold uppercase text-[#8b5cf6]">
              <Shield className="h-3 w-3" /> Validator
            </span>
          )}
        </div>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 py-16 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" /> Loading account…
        </div>
      ) : error ? (
        <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-3 text-sm text-red-200">
          {error}
        </div>
      ) : data ? (
        <>
          <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
            <StatCard label="WAVE balance" value={formatToken(data.wave)} unit="WAVE" icon={Coins} accent="#00d1a7" />
            <StatCard label="USDC balance" value={formatToken(data.usdc)} unit="USDC" icon={Coins} accent="#3b82f6" />
            <StatCard label="Staked" value={formatToken(data.stake)} unit="WAVE" icon={Shield} accent="#8b5cf6" />
            <StatCard label="Rewards" value={formatToken(data.rewards)} unit="WAVE" icon={Layers} accent="#f59e0b" />
          </div>

          <div className="mt-4 grid gap-3 md:grid-cols-2">
            <KV label="WAVE token account" value={shortHash(data.wave_account)} full={data.wave_account} />
            <KV label="USDC token account" value={shortHash(data.usdc_account)} full={data.usdc_account} />
            <KV label="Registered" value={data.registered ? "Yes" : "No"} mono={false} />
            <KV label="Domain" value="DEX_WAVE_USDC" mono={false} link={domainHref("4445585f574156455f5553444300000000000000000000000000000000000000")} />
          </div>

          <div className="mt-8">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
                <Layers className="h-4 w-4 text-[#00d1a7]" /> Shifts
                <span className="rounded bg-white/5 px-1.5 py-0.5 text-[10px] text-slate-400">
                  {data.shift_count}
                </span>
              </h2>
            </div>
            <div className="rounded-xl border border-white/[0.06] bg-[#11161c]">
              <ShiftTable shifts={data.shifts} />
            </div>
            <p className="mt-3 text-xs text-slate-600">
              History is sourced from the live mesh index (latest 200 shifts network-wide). Older
              activity may have rolled off the in-memory window.
            </p>
          </div>
        </>
      ) : null}

      <div className="mt-10">
        <Link href="/explorer/" className="inline-flex items-center gap-1.5 text-xs text-slate-400 hover:text-white">
          <ArrowLeft className="h-3.5 w-3.5" /> Back to explorer
        </Link>
      </div>
    </>
  );
}

export default function AccountPage() {
  return (
    <ExplorerShell>
      <Suspense fallback={<div className="py-16 text-sm text-slate-500">Loading…</div>}>
        <AccountView />
      </Suspense>
    </ExplorerShell>
  );
}
