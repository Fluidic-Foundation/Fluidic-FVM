"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState } from "react";
import { Menu, X } from "lucide-react";

const categories = [
  {
    title: "Getting Started",
    items: [
      { href: "/docs/getting-started/what-is-fluidic", title: "What is Fluidic?" },
      { href: "/docs/getting-started/quickstart", title: "Quickstart" },
      { href: "/docs/getting-started/testnet", title: "Testnet" },
    ],
  },
  {
    title: "Core Concepts",
    items: [
      { href: "/docs/core-concepts/accounts", title: "Accounts" },
      { href: "/docs/core-concepts/shifts", title: "Shifts" },
      { href: "/docs/core-concepts/synthesis-ticks", title: "Synthesis Ticks" },
      { href: "/docs/core-concepts/consensus-staking", title: "Consensus & Staking" },
      { href: "/docs/core-concepts/evm-compatibility", title: "EVM Compatibility" },
    ],
  },
  {
    title: "Tutorials",
    items: [
      { href: "/docs/tutorials/deploy-contract", title: "Deploy a Contract" },
      { href: "/docs/tutorials/build-dapp", title: "Build a dApp" },
    ],
  },
  {
    title: "API Reference",
    items: [
      { href: "/docs/api-reference/rest-api", title: "REST API" },
      { href: "/docs/api-reference/evm-rpc", title: "EVM RPC" },
      { href: "/docs/api-reference/typescript-sdk", title: "TypeScript SDK" },
    ],
  },
];

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [mobileOpen, setMobileOpen] = useState(false);

  const isActive = (href: string) => pathname === href || pathname.startsWith(`${href}/`);

  return (
    <div className="relative min-h-screen bg-[#05050C] text-[#A0A0C5]">
      <nav className="sticky top-0 z-50 border-b border-white/5 bg-[#05050C]/90 backdrop-blur-xl">
        <div className="mx-auto flex h-16 max-w-[1600px] items-center justify-between px-6">
          <Link
            href="/"
            className="group flex items-center gap-3 font-mono text-[12px] uppercase tracking-[0.2em] text-[#A0A0C5] transition-colors hover:text-[#00E6A7]"
          >
            <img
              src="/fluidic-logo-new.png"
              alt="Fluidic"
              className="h-8 w-8 object-contain transition-transform duration-500 group-hover:scale-110"
            />
            <span>Fluidic</span>
          </Link>
          <div className="flex items-center gap-6">
            <div className="hidden items-center gap-8 font-mono text-[11px] uppercase tracking-[0.2em] text-[#A0A0C5] md:flex">
              <Link href="/explorer/" className="transition-colors hover:text-[#00E6A7]">
                Explorer
              </Link>
              <Link href="/docs/" className="text-[#00E6A7]">
                Docs
              </Link>
              <a
                href="https://github.com/Fluidic-Foundation"
                target="_blank"
                rel="noreferrer"
                className="transition-colors hover:text-[#00E6A7]"
              >
                GitHub
              </a>
            </div>
            <button
              className="flex h-9 w-9 items-center justify-center border border-white/10 text-white md:hidden"
              aria-label="Toggle docs menu"
              onClick={() => setMobileOpen((v) => !v)}
            >
              {mobileOpen ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
            </button>
          </div>
        </div>
      </nav>

      <div className="mx-auto flex max-w-[1600px] flex-col gap-8 px-6 py-12 lg:flex-row lg:gap-12">
        {/* Mobile sidebar overlay */}
        {mobileOpen && (
          <div
            className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm lg:hidden"
            onClick={() => setMobileOpen(false)}
          />
        )}

        <aside
          className={[
            "fixed inset-y-0 left-0 z-50 w-72 transform border-r border-white/5 bg-[#05050C]/98 p-6 backdrop-blur-xl transition-transform duration-300 lg:sticky lg:top-24 lg:h-fit lg:w-64 lg:translate-x-0 lg:border-r-0 lg:bg-transparent lg:p-0",
            mobileOpen ? "translate-x-0" : "-translate-x-full",
          ].join(" ")}
        >
          <div className="mb-6 flex items-center justify-between lg:hidden">
            <span className="font-mono text-[12px] uppercase tracking-[0.2em] text-white">
              Docs
            </span>
            <button
              className="flex h-8 w-8 items-center justify-center text-white"
              aria-label="Close docs menu"
              onClick={() => setMobileOpen(false)}
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          <div className="space-y-8">
            {categories.map((category) => (
              <div key={category.title}>
                <h3 className="mb-2 font-mono text-[10px] uppercase tracking-[0.2em] text-white/40">
                  {category.title}
                </h3>
                <div className="space-y-1">
                  {category.items.map((item) => (
                    <Link
                      key={item.href}
                      href={item.href}
                      onClick={() => setMobileOpen(false)}
                      className={[
                        "block border-l border-white/10 py-2 pl-4 font-mono text-[11px] uppercase tracking-[0.12em] transition-all hover:border-[#00E6A7] hover:bg-white/[0.02] hover:text-[#00E6A7]",
                        isActive(item.href)
                          ? "border-[#00E6A7] text-[#00E6A7] bg-white/[0.03]"
                          : "text-[#A0A0C5]",
                      ].join(" ")}
                    >
                      {item.title}
                    </Link>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </aside>

        <main className="min-w-0 flex-1">{children}</main>
      </div>
    </div>
  );
}
