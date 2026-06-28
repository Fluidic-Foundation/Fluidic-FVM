"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import {
  Activity,
  AlertCircle,
  ArrowUpRight,
  CheckCircle2,
  ChevronDown,
  Copy,
  Cpu,
  ExternalLink,
  Globe,
  Layers,
  Loader2,
  Network,
  Search,
  Shield,
  Wallet,
  X,
  XCircle,
  Zap,
} from "lucide-react";

const API_BASE = process.env.NEXT_PUBLIC_FLUIDIC_API || "https://api.testnet.fluidic.foundation";
const WS_BASE = API_BASE.replace(/^http/, (m) => (m === "https" ? "wss" : "ws"));

interface StateResponse {
  wave_reserve: string;
  usdc_reserve: string;
  price: number;
  throughput: number;
  latency_ms: number;
  metabolic_burned: string;
  commutative_applied: number;
  stateful_applied: number;
  evm_applied: number;
  pool_wave_account: string;
  pool_usdc_account: string;
}

interface RecentShift {
  hash: string;
  kind: string;
  status: string;
  domain?: string;
  from?: string;
  to?: string;
  amount?: string;
  timestamp_ns: number;
}

interface RecentTick {
  tick: number;
  hash: string;
  operator: string;
  commutative_applied: number;
  stateful_applied: number;
  evm_applied: number;
  roots: Record<string, string>;
  finalized: boolean;
}

interface OperatorInfo {
  account: string;
  stake: string;
}

interface ShiftStatusResponse {
  hash: string;
  status: "unknown" | "accepted" | "finalized" | "rejected";
  error: string | null;
  synthesis_tick: number;
  confirmations: number;
}

interface TickResponse extends RecentTick {}

const nav = [
  { label: "Dashboard", href: "/explorer" },
  { label: "Ticks", href: "#ticks" },
  { label: "Transactions", href: "#txs" },
  { label: "Validators", href: "#validators" },
  { label: "Docs", href: "/docs" },
];

export default function ExplorerPage() {
  const [state, setState] = useState<StateResponse | null>(null);
  const [shifts, setShifts] = useState<RecentShift[]>([]);
  const [ticks, setTicks] = useState<RecentTick[]>([]);
  const [operators, setOperators] = useState<OperatorInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchResult, setSearchResult] = useState<{ type: "shift" | "tick" | "account"; data: any } | null>(null);
  const [selectedShift, setSelectedShift] = useState<RecentShift | null>(null);

  const totalShifts = (state?.commutative_applied || 0) + (state?.stateful_applied || 0) + (state?.evm_applied || 0);
  const latestTick = ticks[0]?.tick ?? 0;
  const finalizedTick = ticks.find((t) => t.finalized)?.tick ?? latestTick;

  const fetchAll = async () => {
    try {
      const [stateRes, shiftsRes, ticksRes, opsRes] = await Promise.all([
        fetch(`${API_BASE}/api/state`),
        fetch(`${API_BASE}/api/shifts/recent?limit=25`),
        fetch(`${API_BASE}/api/ticks/recent?limit=12`),
        fetch(`${API_BASE}/api/operators`),
      ]);
      if (!stateRes.ok || !shiftsRes.ok || !ticksRes.ok || !opsRes.ok) throw new Error("api");
      const [stateJson, shiftsJson, ticksJson, opsJson] = await Promise.all([
        stateRes.json(),
        shiftsRes.json(),
        ticksRes.json(),
        opsRes.json(),
      ]);
      setState(stateJson);
      setShifts(shiftsJson.shifts || []);
      setTicks(ticksJson.ticks || []);
      setOperators(opsJson.operators || []);
      setError(null);
    } catch (e) {
      setError("Cannot reach the Fluidic mesh. The testnet API may be restarting.");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchAll();
    const id = setInterval(fetchAll, 5_000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    let ws: WebSocket | null = null;
    try {
      ws = new WebSocket(`${WS_BASE}/api/ws`);
      ws.onmessage = (event) => {
        try {
          const snap = JSON.parse(event.data);
          if (snap.throughput !== undefined) {
            setState((prev) =>
              ({
                ...(prev || ({} as StateResponse)),
                wave_reserve: snap.wave_reserve,
                usdc_reserve: snap.usdc_reserve,
                price: snap.price,
                throughput: snap.throughput,
                latency_ms: snap.latency_ms,
                metabolic_burned: snap.metabolic_burned,
                commutative_applied: snap.commutative_applied,
                stateful_applied: snap.stateful_applied,
                evm_applied: snap.evm_applied,
              } as StateResponse)
            );
            setError(null);
          }
        } catch {
          // ignore
        }
      };
      ws.onerror = () => setError("WebSocket disconnected.");
    } catch {
      setError("Live socket unavailable.");
    }
    return () => ws?.close();
  }, []);

  const isHexAccount = (s: string) => /^[0-9a-fA-F]{64}$/.test(s);

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    const q = search.trim();
    if (!q) return;
    setSearching(true);
    setSearchResult(null);
    try {
      if (/^\d+$/.test(q)) {
        const res = await fetch(`${API_BASE}/api/ticks/${q}`);
        if (res.ok) {
          setSearchResult({ type: "tick", data: await res.json() });
        } else {
          setSearchResult({ type: "tick", data: null });
        }
      } else if (isHexAccount(q)) {
        const [balanceRes, shiftRes] = await Promise.all([
          fetch(`${API_BASE}/api/account/${q}/balance`),
          fetch(`${API_BASE}/api/shift/${q}/status`),
        ]);
        const account = balanceRes.ok ? await balanceRes.json() : null;
        const shift = shiftRes.ok ? await shiftRes.json() : null;
        if (account) {
          setSearchResult({ type: "account", data: { account: q, ...account, shift } });
        } else if (shift) {
          setSearchResult({ type: "shift", data: shift });
        } else {
          setSearchResult({ type: "account", data: null });
        }
      } else {
        const res = await fetch(`${API_BASE}/api/shift/${q}/status`);
        if (res.status === 404) {
          setSearchResult({ type: "shift", data: { hash: q, status: "unknown", error: null, synthesis_tick: 0, confirmations: 0 } });
        } else {
          setSearchResult({ type: "shift", data: await res.json() });
        }
      }
    } catch {
      setSearchResult({ type: "shift", data: { hash: search, status: "unknown", error: "Network error", synthesis_tick: 0, confirmations: 0 } });
    }
    setSearching(false);
  };

  return (
    <div className="min-h-screen bg-[#0b0e11] text-slate-300">
      {/* Top nav */}
      <header className="sticky top-0 z-50 border-b border-white/[0.06] bg-[#0b0e11]/95 backdrop-blur">
        <div className="mx-auto flex h-16 max-w-[1500px] items-center gap-6 px-4">
          <Link href="/" className="flex items-center gap-2.5">
            <img src="/fluidic-logo-new.png" alt="" className="h-7 w-7 rounded-full object-cover" />
            <div className="flex flex-col leading-none">
              <span className="text-[15px] font-semibold text-white">Fluidic</span>
              <span className="text-[10px] tracking-wider text-slate-500">EXPLORER</span>
            </div>
          </Link>

          <div className="hidden rounded-lg border border-white/[0.06] bg-[#13171c] px-3 py-1.5 md:flex md:items-center md:gap-2">
            <span className="h-2 w-2 rounded-full bg-[#00d1a7]" />
            <span className="text-xs font-medium text-slate-300">Testnet</span>
            <ChevronDown className="h-3.5 w-3.5 text-slate-500" />
          </div>

          <form onSubmit={handleSearch} className="relative flex-1 max-w-2xl">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search by shift hash, account, or tick number"
              className="h-10 w-full rounded-lg border border-white/[0.06] bg-[#13171c] pl-10 pr-24 text-sm text-white outline-none transition-colors placeholder:text-slate-600 focus:border-[#00d1a7]/50"
            />
            <button
              disabled={searching}
              className="absolute right-1.5 top-1/2 -translate-y-1/2 rounded-md bg-white/5 px-3 py-1 text-xs font-medium text-slate-300 hover:bg-white/10 disabled:opacity-50"
            >
              {searching ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : "Search"}
            </button>
          </form>

          <nav className="hidden items-center gap-5 lg:flex">
            {nav.map((item) => (
              <Link
                key={item.label}
                href={item.href}
                className="text-sm font-medium text-slate-400 transition-colors hover:text-white"
              >
                {item.label}
              </Link>
            ))}
          </nav>
        </div>
      </header>

      {error && (
        <div className="mx-auto max-w-[1500px] px-4 pt-4">
          <div className="flex items-center gap-2 rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-2.5 text-sm text-red-200">
            <AlertCircle className="h-4 w-4" />
            {error}
          </div>
        </div>
      )}

      {/* Stats strip */}
      <section className="mx-auto max-w-[1500px] px-4 py-4">
        <div className="grid grid-cols-2 gap-3 md:grid-cols-4 lg:grid-cols-7">
          <Stat label="Latest tick" value={latestTick ? `#${latestTick.toLocaleString()}` : "—"} icon={Zap} />
          <Stat label="Finalized" value={finalizedTick ? `#${finalizedTick.toLocaleString()}` : "—"} icon={CheckCircle2} />
          <Stat label="Total shifts" value={totalShifts ? totalShifts.toLocaleString() : "—"} icon={Layers} />
          <Stat label="WAV/USDC" value={state ? `$${state.price.toFixed(6)}` : "—"} icon={Activity} />
          <Stat label="Validators" value={operators.length ? operators.length.toString() : "—"} icon={Shield} />
          <Stat label="Throughput" value={state ? `${state.throughput.toFixed(1)}/s` : "—"} icon={Zap} />
          <Stat label="Latency" value={state ? `${state.latency_ms.toFixed(0)}ms` : "—"} icon={Cpu} />
        </div>
      </section>

      {/* Search result */}
      {searchResult && (
        <section className="mx-auto max-w-[1500px] px-4 pb-4">
          <div className="rounded-lg border border-white/[0.06] bg-[#11161c] p-4">
            <div className="mb-2 flex items-center justify-between">
              <h3 className="text-sm font-medium text-white">Search result</h3>
              <button onClick={() => setSearchResult(null)} className="text-xs text-slate-500 hover:text-white">
                Close
              </button>
            </div>
            {searchResult.type === "shift" && searchResult.data ? (
              <div className="grid gap-2 text-sm md:grid-cols-4">
                <KV label="Hash" value={shortHash(searchResult.data.hash)} full={searchResult.data.hash} />
                <KV label="Status" value={<StatusBadge status={searchResult.data.status} />} />
                <KV label="Tick" value={searchResult.data.synthesis_tick} />
                <KV label="Confirmations" value={searchResult.data.confirmations} />
                {searchResult.data.error && <div className="col-span-full text-red-400">{searchResult.data.error}</div>}
              </div>
            ) : searchResult.type === "tick" && searchResult.data ? (
              <div className="grid gap-2 text-sm md:grid-cols-4">
                <KV label="Tick" value={`#${searchResult.data.tick}`} />
                <KV label="Hash" value={shortHash(searchResult.data.hash)} full={searchResult.data.hash} />
                <KV label="Status" value={<StatusBadge status={searchResult.data.finalized ? "finalized" : "accepted"} />} />
                <KV label="Shifts" value={searchResult.data.commutative_applied + searchResult.data.stateful_applied + searchResult.data.evm_applied} />
              </div>
            ) : searchResult.type === "account" && searchResult.data ? (
              <div className="grid gap-2 text-sm md:grid-cols-4">
                <KV label="Account" value={shortHash(searchResult.data.account)} full={searchResult.data.account} />
                <KV label="WAVE balance" value={formatAmount(searchResult.data.wave)} full={searchResult.data.wave} />
                <KV label="USDC balance" value={formatAmount(searchResult.data.usdc)} full={searchResult.data.usdc} />
                {searchResult.data.shift && <KV label="Shift status" value={<StatusBadge status={searchResult.data.shift.status} />} />}
              </div>
            ) : (
              <div className="text-sm text-slate-500">Not found.</div>
            )}
          </div>
        </section>
      )}

      {/* Main content */}
      <main className="mx-auto grid max-w-[1500px] gap-4 px-4 pb-12 lg:grid-cols-3">
        {/* Left: transactions */}
        <section className="lg:col-span-2">
          <div className="rounded-lg border border-white/[0.06] bg-[#11161c]">
            <div className="flex items-center justify-between border-b border-white/[0.06] px-4 py-3">
              <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
                <Layers className="h-4 w-4 text-[#00d1a7]" /> Latest shifts
              </h2>
              <Link href="#" className="flex items-center gap-1 text-xs font-medium text-[#00d1a7] hover:underline">
                View all <ArrowUpRight className="h-3 w-3" />
              </Link>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm">
                <thead className="bg-[#0e1216] text-xs uppercase tracking-wider text-slate-500">
                  <tr>
                    <th className="px-4 py-2.5 font-medium">Hash</th>
                    <th className="px-4 py-2.5 font-medium">Type</th>
                    <th className="px-4 py-2.5 font-medium">From / Domain</th>
                    <th className="px-4 py-2.5 font-medium">To</th>
                    <th className="px-4 py-2.5 font-medium text-right">Amount</th>
                    <th className="px-4 py-2.5 font-medium">Status</th>
                    <th className="px-4 py-2.5 font-medium text-right">Age</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/[0.04]">
                  {loading && shifts.length === 0 ? (
                    <SkeletonRows cols={7} rows={6} />
                  ) : shifts.length === 0 ? (
                    <tr>
                      <td colSpan={7} className="px-4 py-8 text-center text-sm text-slate-500">
                        No shifts indexed yet. Submit a swap or EVM transaction to see live data.
                      </td>
                    </tr>
                  ) : (
                    shifts.map((s, i) => (
                      <tr key={i} className="cursor-pointer transition-colors hover:bg-white/[0.02]" onClick={() => setSelectedShift(s)}>
                        <td className="px-4 py-3 font-mono text-xs text-[#00d1a7]">
                          <div className="flex items-center gap-2">
                            <Link href={`/explorer?shift=${s.hash}`} className="hover:underline">
                              {shortHash(s.hash)}
                            </Link>
                            <CopyButton text={s.hash} />
                          </div>
                        </td>
                        <td className="px-4 py-3">
                          <TypeBadge kind={s.kind} />
                        </td>
                        <td className="px-4 py-3 font-mono text-xs text-slate-400">
                          {s.from ? shortHash(s.from) : s.domain ? shortHash(s.domain) : "—"}
                        </td>
                        <td className="px-4 py-3 font-mono text-xs text-slate-400">{s.to ? shortHash(s.to) : "—"}</td>
                        <td className="px-4 py-3 text-right font-mono text-xs text-slate-300" title={s.amount}>
                          {s.amount ? formatAmount(s.amount) : "—"}
                        </td>
                        <td className="px-4 py-3">
                          <StatusBadge status={s.status as any} />
                        </td>
                        <td className="px-4 py-3 text-right text-xs text-slate-500">
                          {s.timestamp_ns ? timeAgo(s.timestamp_ns / 1_000_000) : "—"}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </section>

        {/* Right: ticks + validators */}
        <aside className="space-y-4">
          {/* Latest ticks */}
          <div className="rounded-lg border border-white/[0.06] bg-[#11161c]" id="ticks">
            <div className="flex items-center justify-between border-b border-white/[0.06] px-4 py-3">
              <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
                <Zap className="h-4 w-4 text-[#3b82f6]" /> Latest ticks
              </h2>
              <Link href="#" className="text-xs font-medium text-[#3b82f6] hover:underline">
                View all
              </Link>
            </div>
            <div className="divide-y divide-white/[0.04]">
              {loading && ticks.length === 0 ? (
                [1, 2, 3, 4, 5].map((n) => (
                  <div key={n} className="px-4 py-3">
                    <div className="h-4 w-24 animate-pulse rounded bg-white/5" />
                    <div className="mt-2 h-3 w-40 animate-pulse rounded bg-white/5" />
                  </div>
                ))
              ) : ticks.length === 0 ? (
                <div className="px-4 py-6 text-center text-sm text-slate-500">No ticks produced yet.</div>
              ) : (
                ticks.map((t) => (
                  <div key={t.tick} className="flex items-center justify-between px-4 py-3 transition-colors hover:bg-white/[0.02]">
                    <div>
                      <div className="flex items-center gap-2">
                        <Link href={`#tick=${t.tick}`} className="text-sm font-semibold text-white hover:text-[#3b82f6]">
                          #{t.tick.toLocaleString()}
                        </Link>
                        {t.finalized && (
                          <span className="rounded bg-[#00d1a7]/10 px-1.5 py-0.5 text-[10px] font-medium text-[#00d1a7]">
                            final
                          </span>
                        )}
                      </div>
                      <div className="mt-0.5 font-mono text-xs text-slate-500">{shortHash(t.hash)}</div>
                    </div>
                    <div className="text-right text-xs text-slate-400">
                      <div>{t.commutative_applied + t.stateful_applied + t.evm_applied} shifts</div>
                      <div className="mt-0.5 font-mono text-[10px] text-slate-500">{shortHash(t.operator)}</div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Validators */}
          <div className="rounded-lg border border-white/[0.06] bg-[#11161c]" id="validators">
            <div className="border-b border-white/[0.06] px-4 py-3">
              <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
                <Shield className="h-4 w-4 text-[#8b5cf6]" /> Validators
              </h2>
            </div>
            <div className="divide-y divide-white/[0.04]">
              {loading && operators.length === 0 ? (
                [1, 2, 3].map((n) => (
                  <div key={n} className="px-4 py-3">
                    <div className="h-3 w-32 animate-pulse rounded bg-white/5" />
                    <div className="mt-2 h-3 w-20 animate-pulse rounded bg-white/5" />
                  </div>
                ))
              ) : operators.length === 0 ? (
                <div className="px-4 py-6 text-center text-sm text-slate-500">No validators online.</div>
              ) : (
                operators.slice(0, 8).map((o, i) => (
                  <div key={i} className="flex items-center justify-between px-4 py-2.5 transition-colors hover:bg-white/[0.02]">
                    <div className="flex items-center gap-2">
                      <span className="flex h-5 w-5 items-center justify-center rounded-full bg-white/5 text-[10px] text-slate-400">
                        {i + 1}
                      </span>
                      <span className="font-mono text-xs text-slate-300">{shortHash(o.account)}</span>
                    </div>
                    <span className="font-mono text-xs text-slate-500" title={o.stake}>{formatAmount(o.stake)}</span>
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Network health */}
          <div className="rounded-lg border border-white/[0.06] bg-[#11161c] p-4">
            <h2 className="mb-3 text-sm font-semibold text-white">Network health</h2>
            <div className="space-y-3">
              <HealthRow label="BFT quorum" ok={operators.some((o) => BigInt(o.stake || 0) > 0)} />
              <HealthRow label="Vector-clock DAG" ok={totalShifts > 0 || ticks.length > 0} />
              <HealthRow label="Live websocket" ok={!error} />
            </div>
          </div>
        </aside>
      </main>

      {/* Shift detail modal */}
      {selectedShift && (
        <div className="fixed inset-0 z-[100] flex items-start justify-center bg-black/70 p-4 pt-24 backdrop-blur-sm" onClick={() => setSelectedShift(null)}>
          <div className="w-full max-w-2xl rounded-lg border border-white/[0.06] bg-[#11161c] p-6" onClick={(e) => e.stopPropagation()}>
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-lg font-semibold text-white">Shift details</h2>
              <button onClick={() => setSelectedShift(null)} className="text-slate-500 hover:text-white">
                <X className="h-5 w-5" />
              </button>
            </div>
            <div className="grid gap-3 text-sm md:grid-cols-2">
              <KV label="Hash" value={shortHash(selectedShift.hash)} full={selectedShift.hash} />
              <KV label="Type" value={<TypeBadge kind={selectedShift.kind} />} />
              <KV label="Status" value={<StatusBadge status={selectedShift.status} />} />
              <KV label="Amount" value={selectedShift.amount ? formatAmount(selectedShift.amount) : "—"} full={selectedShift.amount} />
              <KV label="Domain" value={selectedShift.domain ? decodeDomain(selectedShift.domain) : "—"} full={selectedShift.domain} />
              <KV label="From" value={selectedShift.from ? shortHash(selectedShift.from) : "—"} full={selectedShift.from} />
              <KV label="To" value={selectedShift.to ? shortHash(selectedShift.to) : "—"} full={selectedShift.to} />
              <KV label="Timestamp" value={selectedShift.timestamp_ns ? new Date(selectedShift.timestamp_ns / 1_000_000).toLocaleString() : "—"} />
            </div>
          </div>
        </div>
      )}

      <footer className="border-t border-white/[0.06] bg-[#0b0e11] py-8">
        <div className="mx-auto flex max-w-[1500px] flex-col items-center justify-between gap-3 px-4 text-xs text-slate-500 md:flex-row">
          <div className="flex items-center gap-2">
            <img src="/fluidic-logo-new.png" alt="" className="h-5 w-5 rounded-full opacity-60" />
            <span>Fluidic Explorer · Testnet</span>
          </div>
          <div className="flex gap-5">
            <Link href="/" className="hover:text-white">Home</Link>
            <Link href="/docs/" className="hover:text-white">Docs</Link>
            <a href="https://github.com" className="hover:text-white">GitHub</a>
          </div>
        </div>
      </footer>
    </div>
  );
}

function Stat({ label, value, icon: Icon }: { label: string; value: string; icon: any }) {
  return (
    <div className="rounded-lg border border-white/[0.06] bg-[#11161c] px-3 py-2.5">
      <div className="mb-1 text-[10px] uppercase tracking-wider text-slate-500">{label}</div>
      <div className="flex items-center gap-2">
        <Icon className="h-3.5 w-3.5 text-slate-500" />
        <span className="text-sm font-semibold text-white">{value}</span>
      </div>
    </div>
  );
}

function KV({ label, value, full }: { label: string; value: React.ReactNode; full?: string }) {
  return (
    <div className="rounded bg-white/[0.02] px-3 py-2">
      <div className="text-[10px] uppercase tracking-wider text-slate-500">{label}</div>
      <div className="mt-0.5 flex items-center gap-2 font-mono text-xs text-slate-200" title={full}>
        {value}
        {full && <CopyButton text={full} />}
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: "accepted" | "finalized" | "rejected" | "unknown" | string }) {
  const map: Record<string, { cls: string; icon: any }> = {
    finalized: { cls: "bg-[#00d1a7]/10 text-[#00d1a7]", icon: CheckCircle2 },
    accepted: { cls: "bg-[#3b82f6]/10 text-[#3b82f6]", icon: CheckCircle2 },
    rejected: { cls: "bg-red-500/10 text-red-400", icon: XCircle },
    unknown: { cls: "bg-slate-500/10 text-slate-400", icon: AlertCircle },
  };
  const { cls, icon: Icon } = map[status] || map.unknown;
  return (
    <span className={`inline-flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-semibold uppercase ${cls}`}>
      <Icon className="h-3 w-3" /> {status}
    </span>
  );
}

function TypeBadge({ kind }: { kind: string }) {
  const map: Record<string, { cls: string; icon: any; label: string }> = {
    stateful: { cls: "bg-[#8b5cf6]/10 text-[#8b5cf6]", icon: Network, label: "Stateful" },
    commutative: { cls: "bg-[#f59e0b]/10 text-[#f59e0b]", icon: Layers, label: "Commutative" },
    evm: { cls: "bg-[#ec4899]/10 text-[#ec4899]", icon: Wallet, label: "EVM" },
  };
  const { cls, icon: Icon, label } = map[kind] || { cls: "bg-slate-500/10 text-slate-400", icon: Globe, label: kind };
  return (
    <span className={`inline-flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-semibold ${cls}`}>
      <Icon className="h-3 w-3" /> {label}
    </span>
  );
}

function CopyButton({ text, onClick }: { text: string; onClick?: (e: React.MouseEvent) => void }) {
  const [copied, setCopied] = useState(false);
  const copy = async (e: React.MouseEvent) => {
    e.stopPropagation();
    onClick?.(e);
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1_500);
    } catch {
      // ignore
    }
  };
  return (
    <button onClick={copy} className="text-slate-600 hover:text-[#00d1a7]">
      {copied ? <CheckCircle2 className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
    </button>
  );
}

function SkeletonRows({ cols, rows }: { cols: number; rows: number }) {
  return (
    <>
      {Array.from({ length: rows }).map((_, r) => (
        <tr key={r}>
          {Array.from({ length: cols }).map((_, c) => (
            <td key={c} className="px-4 py-3">
              <div className="h-3 w-full animate-pulse rounded bg-white/5" />
            </td>
          ))}
        </tr>
      ))}
    </>
  );
}

function HealthRow({ label, ok }: { label: string; ok: boolean }) {
  return (
    <div className="flex items-center justify-between text-sm">
      <span className="text-slate-400">{label}</span>
      <span className={`flex items-center gap-1 text-xs font-medium ${ok ? "text-[#00d1a7]" : "text-slate-500"}`}>
        {ok ? <CheckCircle2 className="h-3.5 w-3.5" /> : <AlertCircle className="h-3.5 w-3.5" />}
        {ok ? "Healthy" : "Waiting"}
      </span>
    </div>
  );
}

function shortHash(h: string): string {
  if (!h) return "—";
  if (h.length <= 14) return h;
  return `${h.slice(0, 7)}…${h.slice(-5)}`;
}

function formatAmount(raw: string): string {
  try {
    const n = BigInt(raw);
    if (n === BigInt(0)) return "0";
    const s = n.toString();
    // Show the exact value with comma grouping for anything that fits comfortably.
    if (s.length <= 18) {
      return BigInt(s).toLocaleString("en-US");
    }
    // Fall back to compact notation with 4 significant figures for huge values.
    const suffixes = ["", "K", "M", "B", "T", "P", "E", "Z", "Y"];
    const exp = Math.min(suffixes.length - 1, Math.floor((s.length - 1) / 3));
    const divisor = BigInt("1" + "0".repeat(exp * 3));
    const scaled = Number(n) / Number(divisor);
    const formatted =
      scaled >= 1000
        ? scaled.toFixed(0)
        : scaled >= 100
        ? scaled.toFixed(1)
        : scaled >= 10
        ? scaled.toFixed(2)
        : scaled.toFixed(3);
    return `${formatted.replace(/\.0+$/, "")} ${suffixes[exp]}`;
  } catch {
    return raw;
  }
}

function timeAgo(ms: number): string {
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

function decodeDomain(raw: string): string {
  try {
    const bytes: number[] = [];
    for (let i = 0; i < raw.length; i += 2) {
      bytes.push(parseInt(raw.slice(i, i + 2), 16));
    }
    const text = new TextDecoder().decode(new Uint8Array(bytes));
    return text.replace(/\0/g, "");
  } catch {
    return raw;
  }
}
