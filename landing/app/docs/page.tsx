"use client";

import { motion } from "framer-motion";
import {
  BookOpen,
  Blocks,
  Box,
  Cpu,
  Globe,
  Layers,
  Radio,
  Shield,
  Terminal,
  Wallet,
  Zap,
} from "lucide-react";
import Link from "next/link";

const categories = [
  {
    title: "Getting Started",
    description: "Install a node, run your first shift, and connect to the live testnet.",
    items: [
      { title: "What is Fluidic?", href: "/docs/getting-started/what-is-fluidic", icon: BookOpen, desc: "The blockless, continuous-wave state engine." },
      { title: "Quickstart", href: "/docs/getting-started/quickstart", icon: Terminal, desc: "Docker, SDK, and your first swap in minutes." },
      { title: "Testnet", href: "/docs/getting-started/testnet", icon: Globe, desc: "Live endpoints, faucet, and seed peer." },
    ],
  },
  {
    title: "Core Concepts",
    description: "How Fluidic orders state, reaches consensus, and stays EVM-compatible.",
    items: [
      { title: "Accounts", href: "/docs/core-concepts/accounts", icon: Wallet, desc: "Ed25519 keys, AccountIds, and token accounts." },
      { title: "Shifts", href: "/docs/core-concepts/shifts", icon: Zap, desc: "Stateful vs. commutative operations." },
      { title: "Synthesis Ticks", href: "/docs/core-concepts/synthesis-ticks", icon: Layers, desc: "Oscillator, certificates, and Merkle roots." },
      { title: "Consensus & Staking", href: "/docs/core-concepts/consensus-staking", icon: Shield, desc: "BFT quorum, rewards, and finality." },
      { title: "EVM Compatibility", href: "/docs/core-concepts/evm-compatibility", icon: Blocks, desc: "Run raw Ethereum transactions inside revm." },
    ],
  },
  {
    title: "Tutorials",
    description: "Step-by-step guides for deploying contracts and building frontends.",
    items: [
      { title: "Deploy a Contract", href: "/docs/tutorials/deploy-contract", icon: Box, desc: "Deploy a Counter with Foundry on testnet." },
      { title: "Build a dApp", href: "/docs/tutorials/build-dapp", icon: Cpu, desc: "Connect the TypeScript SDK to a React UI." },
    ],
  },
  {
    title: "API Reference",
    description: "Endpoints, RPC methods, and SDK methods for builders.",
    items: [
      { title: "REST API", href: "/docs/api-reference/rest-api", icon: Radio, desc: "HTTP and WebSocket endpoints." },
      { title: "EVM RPC", href: "/docs/api-reference/evm-rpc", icon: Blocks, desc: "Supported JSON-RPC methods." },
      { title: "TypeScript SDK", href: "/docs/api-reference/typescript-sdk", icon: Terminal, desc: "Client, keypairs, and shift builders." },
    ],
  },
];

export default function DocsHomePage() {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.6 }}
      className="max-w-4xl"
    >
      <header className="mb-12 border-b border-white/10 pb-10">
        <h1 className="font-serif text-4xl font-light leading-[1.05] text-white md:text-6xl">
          Fluidic Documentation
        </h1>
        <p className="mt-4 font-mono text-[12px] leading-relaxed text-[#A0A0C5]">
          The continuous-wave state engine: permissionless nodes, NTT-aggregated commutative shifts,
          vector-clock DAG ordering, and BFT synthesis certificates.
        </p>
      </header>

      <div className="space-y-14">
        {categories.map((category) => (
          <section key={category.title}>
            <h2 className="mb-2 font-serif text-2xl font-light text-[#00E6A7]">
              {category.title}
            </h2>
            <p className="mb-6 font-mono text-[12px] text-[#A0A0C5]">{category.description}</p>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {category.items.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className="group flex flex-col gap-3 border border-white/10 bg-white/[0.02] p-5 transition-all hover:border-[#00E6A7]/50 hover:bg-white/[0.04]"
                >
                  <div className="flex items-center gap-3">
                    <span className="text-[#7024ff] transition-colors group-hover:text-[#00E6A7]">
                      <item.icon className="h-4 w-4" />
                    </span>
                    <span className="font-mono text-[12px] uppercase tracking-[0.12em] text-white transition-colors group-hover:text-[#00E6A7]">
                      {item.title}
                    </span>
                  </div>
                  <p className="font-mono text-[11px] leading-relaxed text-[#A0A0C5]">
                    {item.desc}
                  </p>
                </Link>
              ))}
            </div>
          </section>
        ))}
      </div>

      <footer className="mt-20 border-t border-white/10 pt-10">
        <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-[#A0A0C5]">
          FLUIDIC FOUNDATION — CONTINUOUS-WAVE STATE SYNTHESIS
        </p>
      </footer>
    </motion.div>
  );
}
