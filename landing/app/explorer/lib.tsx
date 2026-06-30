"use client";

import Link from "next/link";
import { useState } from "react";
import { AlertCircle, CheckCircle2, Copy, Globe, Layers, Network, Wallet, XCircle } from "lucide-react";

export const API_BASE =
  process.env.NEXT_PUBLIC_FLUIDIC_API || "https://api.testnet.fluidic.foundation";
export const WS_BASE = API_BASE.replace(/^http/, (m) => (m === "https" ? "wss" : "ws"));

export const WAVE_PRECISION = BigInt("1000000000000"); // 1e12 sub-units per WAVE / USDC

export interface RecentShift {
  hash: string;
  kind: string;
  status: string;
  domain?: string;
  from?: string;
  to?: string;
  amount?: string;
  token?: string;
  timestamp_ns: number;
}

/* ----------------------------- formatting ----------------------------- */

export function shortHash(h?: string): string {
  if (!h) return "—";
  if (h.length <= 14) return h;
  return `${h.slice(0, 8)}…${h.slice(-6)}`;
}

/** Raw sub-unit integer with comma grouping (exact). */
export function formatAmount(raw?: string): string {
  if (raw === undefined || raw === null) return "—";
  try {
    const n = BigInt(raw);
    if (n === BigInt(0)) return "0";
    const s = n.toString();
    if (s.replace("-", "").length <= 18) return n.toLocaleString("en-US");
    const neg = n < BigInt(0);
    const abs = neg ? -n : n;
    const str = abs.toString();
    const suffixes = ["", "K", "M", "B", "T", "P", "E", "Z", "Y"];
    const exp = Math.min(suffixes.length - 1, Math.floor((str.length - 1) / 3));
    const divisor = BigInt("1" + "0".repeat(exp * 3));
    const scaled = Number(abs) / Number(divisor);
    const formatted =
      scaled >= 100 ? scaled.toFixed(1) : scaled >= 10 ? scaled.toFixed(2) : scaled.toFixed(3);
    return `${neg ? "-" : ""}${formatted.replace(/\.0+$/, "")} ${suffixes[exp]}`;
  } catch {
    return raw;
  }
}

/** Convert sub-units → human token amount (divide by 1e12) with up to 4 decimals. */
export function formatToken(raw?: string): string {
  if (raw === undefined || raw === null) return "—";
  try {
    const n = BigInt(raw);
    const neg = n < BigInt(0);
    const abs = neg ? -n : n;
    const whole = abs / WAVE_PRECISION;
    const frac = abs % WAVE_PRECISION;
    let fracStr = frac.toString().padStart(12, "0").slice(0, 4).replace(/0+$/, "");
    const wholeStr = whole.toLocaleString("en-US");
    return `${neg ? "-" : ""}${wholeStr}${fracStr ? "." + fracStr : ""}`;
  } catch {
    return raw;
  }
}

export function timeAgo(ms: number): string {
  const diff = Date.now() - ms;
  if (diff < 0) return "just now";
  const sec = Math.floor(diff / 1000);
  if (sec < 10) return "just now";
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.floor(hr / 24)}d ago`;
}

export function decodeDomain(raw?: string): string {
  if (!raw) return "—";
  try {
    const bytes: number[] = [];
    for (let i = 0; i < raw.length; i += 2) bytes.push(parseInt(raw.slice(i, i + 2), 16));
    const text = new TextDecoder().decode(new Uint8Array(bytes));
    const clean = text.replace(/\0/g, "");
    return clean.length ? clean : raw;
  } catch {
    return raw;
  }
}

export const isHexAccount = (s: string) => /^[0-9a-fA-F]{64}$/.test(s.trim());

/* ----------------------------- components ----------------------------- */

export function CopyButton({ text }: { text?: string }) {
  const [copied, setCopied] = useState(false);
  if (!text) return null;
  const copy = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1_500);
    } catch {
      /* ignore */
    }
  };
  return (
    <button onClick={copy} className="text-slate-600 transition-colors hover:text-[#00d1a7]">
      {copied ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
    </button>
  );
}

export function StatusBadge({ status }: { status: string }) {
  const map: Record<string, { cls: string; icon: any }> = {
    finalized: { cls: "bg-[#00d1a7]/10 text-[#00d1a7]", icon: CheckCircle2 },
    accepted: { cls: "bg-[#3b82f6]/10 text-[#3b82f6]", icon: CheckCircle2 },
    rejected: { cls: "bg-red-500/10 text-red-400", icon: XCircle },
    unknown: { cls: "bg-slate-500/10 text-slate-400", icon: AlertCircle },
  };
  const { cls, icon: Icon } = map[status] || map.unknown;
  return (
    <span
      className={`inline-flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-semibold uppercase ${cls}`}
    >
      <Icon className="h-3 w-3" /> {status}
    </span>
  );
}

export function TypeBadge({ kind }: { kind: string }) {
  const map: Record<string, { cls: string; icon: any; label: string }> = {
    stateful: { cls: "bg-[#8b5cf6]/10 text-[#8b5cf6]", icon: Network, label: "Stateful" },
    commutative: { cls: "bg-[#f59e0b]/10 text-[#f59e0b]", icon: Layers, label: "Commutative" },
    evm: { cls: "bg-[#ec4899]/10 text-[#ec4899]", icon: Wallet, label: "EVM" },
  };
  const { cls, icon: Icon, label } = map[kind] || {
    cls: "bg-slate-500/10 text-slate-400",
    icon: Globe,
    label: kind,
  };
  return (
    <span className={`inline-flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-semibold ${cls}`}>
      <Icon className="h-3 w-3" /> {label}
    </span>
  );
}

/** A labelled key/value row. `link` makes the value a navigable address. */
export function KV({
  label,
  value,
  full,
  mono = true,
  link,
}: {
  label: string;
  value: React.ReactNode;
  full?: string;
  mono?: boolean;
  link?: string;
}) {
  const inner = link ? (
    <Link href={link} className="text-[#00d1a7] hover:underline">
      {value}
    </Link>
  ) : (
    value
  );
  return (
    <div className="rounded-lg bg-white/[0.02] px-3 py-2.5">
      <div className="text-[10px] uppercase tracking-wider text-slate-500">{label}</div>
      <div
        className={`mt-1 flex items-center gap-2 break-all text-xs text-slate-200 ${mono ? "font-mono" : ""}`}
        title={full}
      >
        {inner}
        {full && <CopyButton text={full} />}
      </div>
    </div>
  );
}

export function accountHref(id?: string) {
  return id ? `/explorer/account/?id=${id}` : "#";
}
export function domainHref(id?: string) {
  return id ? `/explorer/domain/?id=${id}` : "#";
}
export function txHref(hash?: string) {
  return hash ? `/explorer/tx/?hash=${hash}` : "#";
}

/* ----------------------------- shared chrome ----------------------------- */

export function ExplorerShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-[#0b0e11] text-slate-300">
      <header className="sticky top-0 z-50 border-b border-white/[0.06] bg-[#0b0e11]/95 backdrop-blur">
        <div className="mx-auto flex h-16 max-w-[1100px] items-center gap-6 px-4">
          <Link href="/explorer/" className="flex items-center gap-2.5">
            <img src="/fluidic-logo-new.png" alt="" className="h-7 w-7 rounded-full object-cover" />
            <div className="flex flex-col leading-none">
              <span className="text-[15px] font-semibold text-white">Fluidic</span>
              <span className="text-[10px] tracking-wider text-slate-500">EXPLORER</span>
            </div>
          </Link>
          <nav className="ml-auto flex items-center gap-5 text-sm">
            <Link href="/explorer/" className="font-medium text-slate-400 hover:text-white">
              Dashboard
            </Link>
            <Link href="/explorer/domain/" className="font-medium text-slate-400 hover:text-white">
              Domains
            </Link>
            <Link href="/docs/" className="font-medium text-slate-400 hover:text-white">
              Docs
            </Link>
          </nav>
        </div>
      </header>
      <main className="mx-auto max-w-[1100px] px-4 py-8">{children}</main>
      <footer className="border-t border-white/[0.06] py-8">
        <div className="mx-auto flex max-w-[1100px] flex-col items-center justify-between gap-3 px-4 text-xs text-slate-500 md:flex-row">
          <div className="flex items-center gap-2">
            <img src="/fluidic-logo-new.png" alt="" className="h-5 w-5 rounded-full opacity-60" />
            <span>Fluidic Explorer · Testnet</span>
          </div>
          <div className="flex gap-5">
            <Link href="/" className="hover:text-white">
              Home
            </Link>
            <Link href="/explorer/" className="hover:text-white">
              Explorer
            </Link>
            <Link href="/docs/" className="hover:text-white">
              Docs
            </Link>
          </div>
        </div>
      </footer>
    </div>
  );
}

export function Crumb({ label }: { label: string }) {
  return (
    <div className="mb-5 flex items-center gap-2 text-xs text-slate-500">
      <Link href="/explorer/" className="hover:text-white">
        Explorer
      </Link>
      <span>/</span>
      <span className="text-slate-300">{label}</span>
    </div>
  );
}

export function ShiftTable({ shifts }: { shifts: RecentShift[] }) {
  if (!shifts.length) {
    return (
      <div className="px-4 py-10 text-center text-sm text-slate-500">
        No shifts found for this view yet.
      </div>
    );
  }
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-left text-sm">
        <thead className="bg-[#0e1216] text-xs uppercase tracking-wider text-slate-500">
          <tr>
            <th className="px-4 py-2.5 font-medium">Hash</th>
            <th className="px-4 py-2.5 font-medium">Type</th>
            <th className="px-4 py-2.5 font-medium">From</th>
            <th className="px-4 py-2.5 font-medium">To</th>
            <th className="px-4 py-2.5 font-medium text-right">Amount</th>
            <th className="px-4 py-2.5 font-medium">Status</th>
            <th className="px-4 py-2.5 font-medium text-right">Age</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-white/[0.04]">
          {shifts.map((s, i) => (
            <tr key={i} className="transition-colors hover:bg-white/[0.02]">
              <td className="px-4 py-3 font-mono text-xs text-[#00d1a7]">
                <Link href={txHref(s.hash)} className="hover:underline">
                  {shortHash(s.hash)}
                </Link>
              </td>
              <td className="px-4 py-3">
                <TypeBadge kind={s.kind} />
              </td>
              <td className="px-4 py-3 font-mono text-xs text-slate-400">
                {s.from ? (
                  <Link href={accountHref(s.from)} className="hover:text-[#00d1a7]">
                    {shortHash(s.from)}
                  </Link>
                ) : s.domain ? (
                  <Link href={domainHref(s.domain)} className="hover:text-[#00d1a7]">
                    {decodeDomain(s.domain)}
                  </Link>
                ) : (
                  "—"
                )}
              </td>
              <td className="px-4 py-3 font-mono text-xs text-slate-400">
                {s.to ? (
                  <Link href={accountHref(s.to)} className="hover:text-[#00d1a7]">
                    {shortHash(s.to)}
                  </Link>
                ) : (
                  "—"
                )}
              </td>
              <td className="px-4 py-3 text-right font-mono text-xs text-slate-300">
                {s.amount ? (
                  <span className="inline-flex items-baseline gap-1.5">
                    {formatToken(s.amount)}
                    <span className="text-[10px] uppercase text-slate-500">{s.token || "units"}</span>
                  </span>
                ) : (
                  "—"
                )}
              </td>
              <td className="px-4 py-3">
                <StatusBadge status={s.status} />
              </td>
              <td className="px-4 py-3 text-right text-xs text-slate-500">
                {s.timestamp_ns ? timeAgo(s.timestamp_ns / 1_000_000) : "—"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
