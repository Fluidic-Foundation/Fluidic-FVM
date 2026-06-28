"use client";

import { useState } from "react";
import Editor from "react-simple-code-editor";
import Prism from "prismjs";
import "prismjs/components/prism-javascript";
import { Play, RotateCcw, FileCode } from "lucide-react";

type FluidicSdk = typeof import("@fluidic-foundation/sdk");

const API_URL = "https://api.testnet.fluidic.foundation";

const TEMPLATES: { label: string; code: string }[] = [
  {
    label: "Swap WAVE → USDC",
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
console.log("status:", status.status);`,
  },
  {
    label: "Stateful transfer",
    code: `const { FluidicClient, FluidicKeypair, buildStatefulShift } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const signer = FluidicKeypair.generate();
const recipient = FluidicKeypair.generate();

await client.register(signer.publicKeyHex);

const shift = buildStatefulShift({
  signer,
  to: recipient.accountId,
  amount: 1_000_000_000n,
  vectorClock: { entries: { [signer.accountId]: 1n } },
});

const { hash } = await client.submitStateful(shift);
console.log("transfer hash:", hash);

const status = await client.shiftStatus(hash);
console.log("status:", status.status);`,
  },
  {
    label: "EVM RPC bridge",
    code: `const { FluidicEvmProvider } = fluidicEvm;
const { FluidicClient } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const evm = new FluidicEvmProvider(client);

const block = await evm.getBlockNumber();
console.log("latest block:", block);

// Replace with any 20-byte Ethereum address
const balance = await evm.getBalance("0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1");
console.log("balance:", balance.toString());`,
  },
  {
    label: "Raw REST call",
    code: `const { FluidicClient } = fluidic;

const client = new FluidicClient({ apiUrl: "${API_URL}" });
const state = await client.state();
console.log("pool reserves:", {
  wave: state.wave_reserve,
  usdc: state.usdc_reserve,
  price: state.price,
});

const validators = await fetch("${API_URL}/api/operators").then(r => r.json());
console.log("validators:", validators);`,
  },
];

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
  const [output, setOutput] = useState<string[]>([]);
  const [running, setRunning] = useState(false);

  const run = async () => {
    if (!sdk) return;
    setRunning(true);
    setOutput(["> Running..."]);
    const lines: string[] = [];
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
      Buffer: undefined,
    };

    try {
      const evmMod = await import("@fluidic-foundation/sdk/evm");
      (globals.fluidicEvm as any).FluidicEvmProvider = evmMod.FluidicEvmProvider;
    } catch {
      // EVM submodule is optional; leave it empty if unavailable.
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
      if (lines.length === 0) lines.push("(no output)");
    } catch (e: any) {
      lines.push(`Error: ${e.message ?? String(e)}`);
    }

    setOutput(lines);
    setRunning(false);
  };

  return (
    <section className="rounded-2xl border border-white/10 bg-white/5 p-6 space-y-4">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
        <h3 className="text-xl font-semibold flex items-center gap-2">
          <FileCode className="w-5 h-5 text-sky-400" /> Live code runner
        </h3>
        <div className="flex flex-wrap gap-2">
          {TEMPLATES.map((t) => (
            <button
              key={t.label}
              onClick={() => setCode(t.code)}
              className="px-3 py-1.5 rounded-md text-xs font-medium bg-white/5 hover:bg-white/10 border border-white/10 transition"
            >
              {t.label}
            </button>
          ))}
        </div>
      </div>

      <p className="text-sm text-white/50">
        Write TypeScript-flavored JavaScript against the SDK. Imports from{" "}
        <code>@fluidic-foundation/sdk</code> are injected automatically.
      </p>

      <div className="rounded-xl border border-white/10 bg-black/60 overflow-hidden font-mono text-sm">
        <Editor
          value={code}
          onValueChange={setCode}
          highlight={(code) =>
            Prism.highlight(code, Prism.languages.javascript, "javascript")
          }
          padding={16}
          textareaClassName="!outline-none !bg-transparent"
          className="min-h-[280px] text-white/90"
          style={{
            fontFamily: '"Fira Code", "Monaco", "Consolas", monospace',
            fontSize: 13,
            lineHeight: "1.5",
          }}
        />
      </div>

      <div className="flex gap-2">
        <button
          onClick={run}
          disabled={running || !sdk}
          className="px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 disabled:opacity-50 text-black font-medium text-sm transition flex items-center gap-2"
        >
          <Play className="w-4 h-4" />
          {running ? "Running..." : "Run code"}
        </button>
        <button
          onClick={() => setOutput([])}
          className="px-4 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white font-medium text-sm transition flex items-center gap-2"
        >
          <RotateCcw className="w-4 h-4" /> Clear
        </button>
      </div>

      {output.length > 0 && (
        <div className="rounded-xl bg-black/60 border border-white/10 p-4">
          <p className="text-xs uppercase tracking-wider text-white/40 mb-2">Console output</p>
          <pre className="font-mono text-xs text-white/80 whitespace-pre-wrap">
            {output.join("\n")}
          </pre>
        </div>
      )}
    </section>
  );
}
