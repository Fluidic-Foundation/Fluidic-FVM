"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { ArrowLeft, CheckCircle2, FileText, Loader2 } from "lucide-react";
import {
  API_BASE,
  Crumb,
  ExplorerShell,
  KV,
  RecentShift,
  StatusBadge,
  TypeBadge,
  accountHref,
  decodeDomain,
  domainHref,
  formatToken,
  shortHash,
} from "../lib";

interface ShiftStatus {
  hash: string;
  status: string;
  error: string | null;
  synthesis_tick: number;
  confirmations: number;
}

const ERROR_HELP: Record<string, string> = {
  missing_predecessor: "A causal predecessor referenced in the vector clock was not found.",
  invalid_signature: "The signature did not verify against the sender's key.",
  insufficient_balance: "The sender did not hold enough balance (after fees) to settle.",
  double_spend: "A conflicting shift consumed the same causal slot first.",
  causal_cycle: "The shift introduced a cycle in the vector-clock DAG.",
};

function TxView() {
  const params = useSearchParams();
  const hash = (params.get("hash") || "").trim();
  const [status, setStatus] = useState<ShiftStatus | null>(null);
  const [detail, setDetail] = useState<RecentShift | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!hash || !/^[0-9a-fA-F]{64}$/.test(hash)) {
      setError("Invalid shift hash — expected 64 hex characters.");
      setLoading(false);
      return;
    }
    let active = true;
    const load = async () => {
      try {
        const [statusRes, recentRes] = await Promise.all([
          fetch(`${API_BASE}/api/shift/${hash}/status`),
          fetch(`${API_BASE}/api/shifts/recent?limit=200`),
        ]);
        const st = statusRes.ok
          ? await statusRes.json()
          : { hash, status: "unknown", error: null, synthesis_tick: 0, confirmations: 0 };
        let match: RecentShift | null = null;
        if (recentRes.ok) {
          const j = await recentRes.json();
          match = (j.shifts || []).find((s: RecentShift) => s.hash === hash) || null;
        }
        if (active) {
          setStatus(st);
          setDetail(match);
          setError(null);
        }
      } catch {
        if (active) setError("Could not load this shift from the mesh.");
      } finally {
        if (active) setLoading(false);
      }
    };
    load();
    const t = setInterval(load, 5_000);
    return () => {
      active = false;
      clearInterval(t);
    };
  }, [hash]);

  return (
    <>
      <Crumb label="Shift" />
      <div className="mb-6 flex items-start gap-4">
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#00d1a7]/10">
          <FileText className="h-6 w-6 text-[#00d1a7]" />
        </div>
        <div className="min-w-0">
          <h1 className="text-lg font-semibold text-white">Shift</h1>
          <div className="mt-0.5 break-all font-mono text-xs text-slate-400">{hash || "—"}</div>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 py-16 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" /> Loading shift…
        </div>
      ) : error ? (
        <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-3 text-sm text-red-200">
          {error}
        </div>
      ) : status ? (
        <>
          {/* status banner */}
          <div className="mb-5 flex flex-wrap items-center gap-3 rounded-xl border border-white/[0.06] bg-[#11161c] p-4">
            <StatusBadge status={status.status} />
            {detail && <TypeBadge kind={detail.kind} />}
            <div className="ml-auto flex items-center gap-1.5 text-xs text-slate-400">
              <CheckCircle2 className="h-3.5 w-3.5 text-[#00d1a7]" />
              {status.confirmations} confirmation{status.confirmations === 1 ? "" : "s"}
            </div>
          </div>

          {status.error && (
            <div className="mb-5 rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-3 text-sm text-red-200">
              <span className="font-semibold">Rejected: {status.error}</span>
              {ERROR_HELP[status.error] && (
                <span className="mt-1 block text-red-300/80">{ERROR_HELP[status.error]}</span>
              )}
            </div>
          )}

          <div className="grid gap-3 md:grid-cols-2">
            <KV label="Status" value={<StatusBadge status={status.status} />} />
            <KV label="Synthesis tick" value={`#${status.synthesis_tick.toLocaleString()}`} mono={false} />
            <KV label="Confirmations" value={String(status.confirmations)} mono={false} />
            {detail && (
              <KV label="Type" value={<TypeBadge kind={detail.kind} />} />
            )}
            {detail?.from && (
              <KV label="From" value={shortHash(detail.from)} full={detail.from} link={accountHref(detail.from)} />
            )}
            {detail?.to && (
              <KV label="To" value={shortHash(detail.to)} full={detail.to} link={accountHref(detail.to)} />
            )}
            {detail?.amount && (
              <KV
                label="Amount"
                value={`${formatToken(detail.amount)} ${detail.token || "units"}`}
                mono={false}
              />
            )}
            {detail?.domain && (
              <KV label="Domain" value={decodeDomain(detail.domain)} mono={false} link={domainHref(detail.domain)} />
            )}
            {detail?.timestamp_ns ? (
              <KV
                label="Timestamp"
                value={new Date(detail.timestamp_ns / 1_000_000).toLocaleString()}
                mono={false}
              />
            ) : null}
          </div>

          {!detail && status.status !== "rejected" && (
            <p className="mt-4 text-xs text-slate-600">
              This shift is no longer in the live recent-shift window, so sender/amount details are
              unavailable. Its consensus status above is read directly from the DAG.
            </p>
          )}
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

export default function TxPage() {
  return (
    <ExplorerShell>
      <Suspense fallback={<div className="py-16 text-sm text-slate-500">Loading…</div>}>
        <TxView />
      </Suspense>
    </ExplorerShell>
  );
}
