"use client";

import { useMemo, useState } from "react";
import Editor from "react-simple-code-editor";
import Prism from "prismjs";
import "prismjs/components/prism-javascript";
import {
  Play,
  RotateCcw,
  Code2,
  ChevronDown,
  Terminal,
  CheckCircle2,
  Circle,
  Trash2,
  Zap,
} from "lucide-react";

type FluidicSdk = typeof import("@fluidic-foundation/sdk");

const API_URL = "https://api.testnet.fluidic.foundation";

const TEMPLATES: { label: string; description: string; code: string }[] = [
  {
    label: "Swap WAVE → USDC",
    description: "Classic single-account swap against the pool.",
    code: `const { FluidicClient, FluidicKeypair, submitSwap } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const signer = FluidicKeypair.generate();

// Register and fund the account (testnet faucet)
await client.register(signer.publicKeyHex);

// Swap 1 WAVE for USDC
const result = await submitSwap(client, {
  signer,
  direction: "WAVE_TO_USDC",
  amount: 1_000_000_000_000n,
  vectorClock: { entries: { [signer.waveAccount]: 1n } },
});

console.log("pool-in hash:", result.poolInHash);

const status = await client.shiftStatus(result.poolInHash);
console.log("status:", status.status, "tick:", status.synthesis_tick);`,
  },
  {
    label: "Parallel swaps",
    description: "Two independent accounts swap at the same time.",
    code: `const { FluidicClient, FluidicKeypair, submitSwap } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const alice = FluidicKeypair.generate();
const bob = FluidicKeypair.generate();

const start = performance.now();
await Promise.all([
  client.register(alice.publicKeyHex),
  client.register(bob.publicKeyHex),
]);
console.log("funded 2 accounts in", Math.round(performance.now() - start), "ms");

// Submit both swaps concurrently — no nonce collision because each account
// lives on its own wave-field thread.
const [r1, r2] = await Promise.all([
  submitSwap(client, {
    signer: alice,
    direction: "WAVE_TO_USDC",
    amount: 1_000_000_000_000n,
    vectorClock: { entries: { [alice.waveAccount]: 1n } },
  }),
  submitSwap(client, {
    signer: bob,
    direction: "WAVE_TO_USDC",
    amount: 2_000_000_000_000n,
    vectorClock: { entries: { [bob.waveAccount]: 1n } },
  }),
]);
console.log("parallel pool-in hashes:", r1.poolInHash, r2.poolInHash);

const [s1, s2] = await Promise.all([
  client.waitForFinalization(r1.poolInHash),
  client.waitForFinalization(r2.poolInHash),
]);
console.log(
  "both finalized in",
  Math.round(performance.now() - start),
  "ms —",
  s1.status,
  "&",
  s2.status
);`,
  },
  {
    label: "Time to finality",
    description: "Measure how fast a swap is finalized by the mesh.",
    code: `const { FluidicClient, FluidicKeypair, submitSwap } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const signer = FluidicKeypair.generate();
await client.register(signer.publicKeyHex);

const start = performance.now();
const result = await submitSwap(client, {
  signer,
  direction: "WAVE_TO_USDC",
  amount: 1_000_000_000_000n,
  vectorClock: { entries: { [signer.waveAccount]: 1n } },
});
console.log("submitted:", result.poolInHash);

const status = await client.waitForFinalization(result.poolInHash);
const elapsed = Math.round(performance.now() - start);
console.log(
  "finalized in",
  elapsed,
  "ms | status:",
  status.status,
  "| tick:",
  status.synthesis_tick
);`,
  },
  {
    label: "Live state snapshots",
    description: "Subscribe to real-time pool snapshots over WebSocket.",
    code: `const { FluidicClient, FluidicKeypair, submitSwap } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const signer = FluidicKeypair.generate();
await client.register(signer.publicKeyHex);

let snapshots = 0;
const unsubscribe = client.subscribeSnapshots((snap) => {
  snapshots++;
  console.log("snapshot tick:", snap.tick, "wave reserve:", snap.wave_reserve);
});

// Give the socket a moment to connect, then submit a swap.
await new Promise((r) => setTimeout(r, 600));
const result = await submitSwap(client, {
  signer,
  direction: "WAVE_TO_USDC",
  amount: 1_000_000_000_000n,
  vectorClock: { entries: { [signer.waveAccount]: 1n } },
});
await client.waitForFinalization(result.poolInHash);
await new Promise((r) => setTimeout(r, 600));
unsubscribe();

console.log("total snapshots received:", snapshots);`,
  },
  {
    label: "Quorum consensus",
    description: "Wait for a synthesis quorum to form for a tick.",
    code: `const { FluidicClient, FluidicKeypair, submitSwap } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const signer = FluidicKeypair.generate();
await client.register(signer.publicKeyHex);

const result = await submitSwap(client, {
  signer,
  direction: "WAVE_TO_USDC",
  amount: 1_000_000_000_000n,
  vectorClock: { entries: { [signer.waveAccount]: 1n } },
});

const status = await client.shiftStatus(result.poolInHash);
console.log("shift status:", status.status, "| tick:", status.synthesis_tick);

if (status.synthesis_tick) {
  const quorum = await client.waitForQuorum(status.synthesis_tick);
  console.log(
    "quorum finalized:",
    quorum.finalized,
    "| total stake:",
    quorum.total_stake,
    "| threshold:",
    quorum.threshold
  );
}`,
  },
  {
    label: "EVM RPC bridge",
    description: "Call the Fluidic EVM gateway over standard JSON-RPC.",
    code: `const { FluidicEvmProvider } = fluidicEvm;
const { FluidicClient } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const evm = new FluidicEvmProvider(client);

const block = await evm.blockNumber();
console.log("latest block:", block.toString());

const chainId = await evm.chainId();
console.log("chain id:", chainId.toString());

// Replace with any 20-byte Ethereum address
const balance = await evm.getBalance("0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1");
console.log("balance:", balance.toString());`,
  },
];

const FONT = '"JetBrains Mono", "Fira Code", "Monaco", "Consolas", monospace';
const LINE_HEIGHT = 20;
const PADDING = { top: 16, right: 16, bottom: 16, left: 52 };

function formatArg(arg: unknown): string {
  if (arg === undefined) return "undefined";
  if (arg === null) return "null";
  if (typeof arg === "bigint") return `${arg.toString()}n`;
  if (typeof arg === "object") {
    try {
      return JSON.stringify(
        arg,
        (_k, v) => (typeof v === "bigint" ? `${v.toString()}n` : v),
        2
      );
    } catch {
      return String(arg);
    }
  }
  return String(arg);
}

export function CodeRunner({ sdk }: { sdk: FluidicSdk }) {
  const [code, setCode] = useState(TEMPLATES[0].code);
  const [template, setTemplate] = useState(TEMPLATES[0].label);
  const [output, setOutput] = useState<string[]>([]);
  const [running, setRunning] = useState(false);

  const lineCount = useMemo(() => code.split("\n").length, [code]);

  const loadTemplate = (label: string) => {
    const t = TEMPLATES.find((t) => t.label === label);
    if (!t) return;
    setTemplate(label);
    setCode(t.code);
  };

  const run = async () => {
    if (!sdk) return;
    setRunning(true);
    const lines: string[] = [`> ${new Date().toLocaleTimeString()}  Running script...`];
    const log = (...args: unknown[]) =>
      lines.push(args.map(formatArg).join(" "));

    const globals: Record<string, unknown> = {
      fluidic: {
        FluidicClient: sdk.FluidicClient,
        FluidicKeypair: sdk.FluidicKeypair,
        submitSwap: sdk.submitSwap,
        quoteSwap: sdk.quoteSwap,
        buildStatefulShift: sdk.buildStatefulShift,
        buildCommutativeShift: sdk.buildCommutativeShift,
      },
      fluidicEvm: {},
      console: { log, error: log, warn: log, info: log },
      fetch,
      setTimeout,
      clearTimeout,
      BigInt,
      Number,
      String,
      Array,
      Object,
      JSON,
      Math,
      Date,
      Promise,
      Error,
      performance:
        typeof performance !== "undefined"
          ? performance
          : { now: () => Date.now() },
      Buffer: undefined,
    };

    try {
      const evmMod = await import("@fluidic-foundation/sdk/evm");
      (globals.fluidicEvm as any).FluidicEvmProvider = evmMod.FluidicEvmProvider;
    } catch {
      // EVM submodule is optional.
    }

    let script = code
      .replace(
        /import\s+{([^}]+)}\s+from\s+["']@fluidic-foundation\/sdk["'];?/g,
        "const {$1} = fluidic;"
      )
      .replace(
        /import\s+{([^}]+)}\s+from\s+["']@fluidic-foundation\/sdk\/evm["'];?/g,
        "const {$1} = fluidicEvm;"
      );

    script = `(async () => {\n${script}\n})()`;

    try {
      const fn = new Function("globals", `with (globals) { return ${script}; }`);
      await fn(globals);
      if (lines.length === 1) lines.push("(no output)");
    } catch (e: any) {
      lines.push(`Error: ${e.message ?? String(e)}`);
    }

    setOutput(lines);
    setRunning(false);
  };

  const activeTemplate = TEMPLATES.find((t) => t.label === template);

  return (
    <section className="rounded-2xl border border-white/10 bg-neutral-900/40 backdrop-blur-sm p-4 sm:p-5 shadow-sm space-y-4">
      {/* Toolbar */}
      <div className="flex flex-col xl:flex-row xl:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <Code2 className="w-5 h-5 text-emerald-400" />
            <h3 className="text-lg font-semibold tracking-tight text-slate-100">SDK Playground</h3>
          </div>
          <span className="hidden sm:inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-xs text-emerald-400">
            <CheckCircle2 className="w-3 h-3" /> SDK loaded
          </span>
        </div>

        <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-3">
          <div className="relative flex-1 sm:flex-none min-w-0">
            <select
              value={template}
              onChange={(e) => loadTemplate(e.target.value)}
              className="w-full sm:w-auto appearance-none bg-neutral-950 border border-neutral-800 hover:border-neutral-700 rounded-lg pl-3 pr-9 py-2 text-sm text-slate-200 focus:outline-none focus:border-emerald-500 transition truncate"
            >
              {TEMPLATES.map((t) => (
                <option key={t.label} value={t.label}>
                  {t.label}
                </option>
              ))}
            </select>
            <ChevronDown className="w-4 h-4 text-slate-500 absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={run}
              disabled={running || !sdk}
              className="flex-1 sm:flex-none px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 disabled:opacity-50 text-black font-medium text-sm transition flex items-center justify-center gap-2"
            >
              <Play className="w-4 h-4 fill-current" />
              {running ? "Running..." : "Run"}
            </button>

            <button
              onClick={() => setOutput([])}
              disabled={output.length === 0}
              className="px-3 py-2 rounded-lg bg-white/5 hover:bg-white/10 disabled:opacity-40 text-slate-300 text-sm font-medium transition flex items-center gap-2"
            >
              <Trash2 className="w-4 h-4" /> <span className="hidden sm:inline">Clear</span>
            </button>
          </div>
        </div>
      </div>

      {activeTemplate && (
        <p className="text-sm text-slate-400 flex items-start gap-2">
          <Zap className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
          {activeTemplate.description}
        </p>
      )}

      {/* IDE shell */}
      <div className="rounded-xl border border-neutral-800 bg-[#0d1117] overflow-hidden flex flex-col h-[420px] sm:h-[520px] md:h-[600px] min-h-[320px]">
        {/* Tabs */}
        <div className="flex items-center justify-between px-3 border-b border-white/5 bg-neutral-950/50 overflow-x-auto">
          <div className="flex items-center">
            <div className="px-3 py-2 text-xs font-medium text-slate-200 border-r border-white/5 bg-white/5 flex items-center gap-2 whitespace-nowrap">
              <Circle className="w-2 h-2 fill-amber-400 text-amber-400" />
              script.js
            </div>
            <div className="px-3 py-2 text-xs text-slate-500 whitespace-nowrap">SDK globals are pre-loaded</div>
          </div>
          <div className="px-3 text-xs text-slate-500 whitespace-nowrap">{lineCount} lines</div>
        </div>

        {/* Editor */}
        <div className="relative flex-1 overflow-auto">
          {/* Line numbers */}
          <div
            className="absolute left-0 top-0 bottom-0 w-12 select-none text-right pr-3 text-slate-600 text-xs font-mono bg-[#0d1117] border-r border-white/5"
            style={{ fontFamily: FONT, lineHeight: `${LINE_HEIGHT}px`, paddingTop: PADDING.top }}
          >
            {Array.from({ length: lineCount }, (_, i) => (
              <div key={i}>{i + 1}</div>
            ))}
          </div>

          <Editor
            value={code}
            onValueChange={setCode}
            highlight={(code) =>
              Prism.highlight(code, Prism.languages.javascript, "javascript")
            }
            padding={PADDING}
            textareaClassName="!outline-none !bg-transparent"
            className="min-h-full text-slate-200"
            style={{
              fontFamily: FONT,
              fontSize: 13,
              lineHeight: `${LINE_HEIGHT}px`,
            }}
          />
        </div>

        {/* Console */}
        <div className="h-32 sm:h-40 border-t border-white/5 bg-neutral-950/80 flex flex-col shrink-0">
          <div className="flex items-center justify-between px-4 py-2 border-b border-white/5 bg-neutral-950/60">
            <div className="flex items-center gap-2 text-xs font-medium text-slate-400 uppercase tracking-wider">
              <Terminal className="w-3.5 h-3.5" /> Console
            </div>
            <button
              onClick={() => setOutput([])}
              disabled={output.length === 0}
              className="text-xs text-slate-500 hover:text-slate-300 disabled:opacity-40 flex items-center gap-1 transition"
            >
              <RotateCcw className="w-3 h-3" /> Clear
            </button>
          </div>
          <div className="flex-1 overflow-auto p-4">
            {output.length === 0 ? (
              <p className="text-xs text-slate-600 italic">Run the script to see output here.</p>
            ) : (
              <pre className="font-mono text-xs text-slate-300 whitespace-pre-wrap">
                {output.join("\n")}
              </pre>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
