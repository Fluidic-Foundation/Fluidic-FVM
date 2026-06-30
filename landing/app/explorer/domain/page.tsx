"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { ArrowLeft, Boxes, GitBranch, Layers, Loader2, Percent, Timer, Waves } from "lucide-react";
import {
  API_BASE,
  Crumb,
  ExplorerShell,
  KV,
  RecentShift,
  ShiftTable,
  domainHref,
  shortHash,
} from "../lib";

interface FeePolicy {
  type: string;
  label: string;
  fee?: string;
  basis_points?: number;
  percent?: number;
}
interface Domain {
  id: string;
  name: string;
  commutative: boolean;
  stateful: boolean;
  ordering: string;
  finalization_depth: number;
  metabolic_lambda_bp: number;
  fee_policy: FeePolicy;
  shift_count: number;
  recent_shifts?: RecentShift[];
}

function feeText(fp: FeePolicy): string {
  if (fp.type === "flat") return `Flat · ${fp.fee} sub-units`;
  if (fp.type === "percentage") return `${fp.percent}% (${fp.basis_points} bp)`;
  return "Metabolic only";
}

/** λ in bp/tick → effective per-tick retention and a friendly half-life. */
function decayText(lambda_bp: number): string {
  if (lambda_bp <= 0) return "No decay";
  const retain = (10_000 - lambda_bp) / 10_000;
  // half-life in ticks: t where retain^t = 0.5
  const halfLife = Math.log(0.5) / Math.log(retain);
  return `${lambda_bp} bp/tick · half-life ≈ ${Math.round(halfLife).toLocaleString()} ticks`;
}

function Pill({ ok, label }: { ok: boolean; label: string }) {
  return (
    <span
      className={`inline-flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-semibold uppercase ${
        ok ? "bg-[#00d1a7]/10 text-[#00d1a7]" : "bg-slate-500/10 text-slate-500"
      }`}
    >
      {label}: {ok ? "yes" : "no"}
    </span>
  );
}

function DomainCard({ d }: { d: Domain }) {
  return (
    <Link
      href={domainHref(d.id)}
      className="block rounded-xl border border-white/[0.06] bg-[#11161c] p-5 transition-colors hover:border-[#00d1a7]/40"
    >
      <div className="flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-[#00d1a7]/10">
          <Boxes className="h-5 w-5 text-[#00d1a7]" />
        </div>
        <div className="min-w-0">
          <div className="font-semibold text-white">{d.name}</div>
          <div className="truncate font-mono text-[11px] text-slate-500">{shortHash(d.id)}</div>
        </div>
        <div className="ml-auto text-right">
          <div className="text-sm font-semibold text-white">{d.shift_count.toLocaleString()}</div>
          <div className="text-[10px] uppercase text-slate-500">recent shifts</div>
        </div>
      </div>
      <div className="mt-4 flex flex-wrap gap-1.5">
        <Pill ok={d.commutative} label="commutative" />
        <Pill ok={d.stateful} label="stateful" />
        <span className="inline-flex items-center gap-1 rounded bg-[#3b82f6]/10 px-2 py-0.5 text-[10px] font-semibold uppercase text-[#3b82f6]">
          {d.ordering}
        </span>
        <span className="inline-flex items-center gap-1 rounded bg-[#f59e0b]/10 px-2 py-0.5 text-[10px] font-semibold uppercase text-[#f59e0b]">
          {d.fee_policy.label}
        </span>
      </div>
    </Link>
  );
}

function DomainList() {
  const [domains, setDomains] = useState<Domain[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const res = await fetch(`${API_BASE}/api/domains`);
        if (!res.ok) throw new Error("api");
        const json = await res.json();
        if (active) {
          setDomains(json.domains || []);
          setError(null);
        }
      } catch {
        if (active) setError("Could not load domains.");
      } finally {
        if (active) setLoading(false);
      }
    };
    load();
    const t = setInterval(load, 8_000);
    return () => {
      active = false;
      clearInterval(t);
    };
  }, []);

  return (
    <>
      <Crumb label="Domains" />
      <div className="mb-6 flex items-center gap-3">
        <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-[#00d1a7]/10">
          <Boxes className="h-6 w-6 text-[#00d1a7]" />
        </div>
        <div>
          <h1 className="text-lg font-semibold text-white">Concurrency domains</h1>
          <p className="text-sm text-slate-500">
            Isolated execution namespaces, each with its own ordering, fee policy, and metabolic
            decay constant.
          </p>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 py-16 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" /> Loading domains…
        </div>
      ) : error ? (
        <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-3 text-sm text-red-200">
          {error}
        </div>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {domains.map((d) => (
            <DomainCard key={d.id} d={d} />
          ))}
        </div>
      )}
    </>
  );
}

function DomainDetail({ id }: { id: string }) {
  const [d, setD] = useState<Domain | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const res = await fetch(`${API_BASE}/api/domain/${id}`);
        if (res.status === 404) throw new Error("notfound");
        if (!res.ok) throw new Error("api");
        const json = await res.json();
        if (active) {
          setD(json);
          setError(null);
        }
      } catch (e: any) {
        if (active) setError(e?.message === "notfound" ? "Domain not found." : "Could not load domain.");
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
      <Crumb label="Domain" />
      {loading ? (
        <div className="flex items-center gap-2 py-16 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" /> Loading domain…
        </div>
      ) : error ? (
        <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-3 text-sm text-red-200">
          {error}
        </div>
      ) : d ? (
        <>
          <div className="mb-6 flex items-start gap-4">
            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#00d1a7]/10">
              <Boxes className="h-6 w-6 text-[#00d1a7]" />
            </div>
            <div className="min-w-0">
              <h1 className="text-lg font-semibold text-white">{d.name}</h1>
              <div className="mt-0.5 break-all font-mono text-xs text-slate-400">{d.id}</div>
            </div>
          </div>

          <div className="grid gap-3 md:grid-cols-3">
            <KV label="Ordering" value={d.ordering.toUpperCase()} mono={false} />
            <KV label="Finalization depth" value={`${d.finalization_depth} ticks`} mono={false} />
            <KV label="Recent shifts" value={d.shift_count.toLocaleString()} mono={false} />
            <KV label="Commutative signals" value={d.commutative ? "Allowed" : "Disabled"} mono={false} />
            <KV label="Stateful signals" value={d.stateful ? "Allowed" : "Disabled"} mono={false} />
            <KV label="Fee policy" value={feeText(d.fee_policy)} mono={false} />
          </div>

          <div className="mt-4 rounded-xl border border-white/[0.06] bg-[#11161c] p-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-white">
              <Waves className="h-4 w-4 text-[#8b5cf6]" /> Metabolic decay
            </div>
            <p className="mt-2 text-sm text-slate-400">{decayText(d.metabolic_lambda_bp)}</p>
            <p className="mt-1 font-mono text-xs text-slate-600">
              B(t) = B(0) · ((10000 − {d.metabolic_lambda_bp}) / 10000)^t
            </p>
          </div>

          <div className="mt-8">
            <h2 className="mb-3 flex items-center gap-2 text-sm font-semibold text-white">
              <Layers className="h-4 w-4 text-[#00d1a7]" /> Latest shifts in this domain
            </h2>
            <div className="rounded-xl border border-white/[0.06] bg-[#11161c]">
              <ShiftTable shifts={d.recent_shifts || []} />
            </div>
          </div>
        </>
      ) : null}

      <div className="mt-10">
        <Link href="/explorer/domain/" className="inline-flex items-center gap-1.5 text-xs text-slate-400 hover:text-white">
          <ArrowLeft className="h-3.5 w-3.5" /> All domains
        </Link>
      </div>
    </>
  );
}

function DomainRouter() {
  const params = useSearchParams();
  const id = (params.get("id") || "").trim();
  return id ? <DomainDetail id={id} /> : <DomainList />;
}

export default function DomainPage() {
  return (
    <ExplorerShell>
      <Suspense fallback={<div className="py-16 text-sm text-slate-500">Loading…</div>}>
        <DomainRouter />
      </Suspense>
    </ExplorerShell>
  );
}
