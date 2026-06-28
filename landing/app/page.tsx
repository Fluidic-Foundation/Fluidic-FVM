"use client";

import {
  AppWindow,
  ArrowRight,
  Blocks,
  BookOpen,
  Box,
  ChevronDown,
  Cpu,
  Globe,
  Layers,
  Menu,
  Network,
  Radio,
  Shield,
  Terminal,
  Wallet,
  X,
  Zap,
} from "lucide-react";
import Link from "next/link";
import { useState } from "react";

export default function FluidicPage() {
  const [mobileOpen, setMobileOpen] = useState(false);
  const WAVE_PIXELS = Array.from({ length: 80 });

  return (
    <div className="infinite-matrix-canvas">
      {/* Background Ambient Glows */}
      <div className="ambient-glow glow-1"></div>
      <div className="ambient-glow glow-2"></div>
      <div className="ambient-glow glow-3"></div>

      <div className="vertical-label">CONTINUOUS STATE SYNTHESIS</div>

      <nav className="floating-dock">
        <div className="dock-left">
          <Link href="/" className="brand-container">
            <img src="/fluidic-logo-new.png" className="brand-logo" alt="Fluidic Logo" />
            <span className="brand-name">Fluidic</span>
          </Link>
          <div className="dock-links">
            <NavDropdown label="Use Fluidic">
              <DropdownCta
                title="Get started"
                desc="Learn how Fluidic replaces blocks with a continuous wave-field and how anyone can run a node."
                href="/docs/"
              />
              <DropdownGrid>
                <DropdownSection title="For users">
                  <DropdownItem icon={BookOpen} href="/docs/getting-started/what-is-fluidic" title="What is Fluidic?" desc="The blockless execution model in plain terms." />
                  <DropdownItem icon={Wallet} href="/docs/core-concepts/accounts" title="Accounts & keys" desc="Ed25519 identities and derived token accounts." />
                  <DropdownItem icon={Cpu} href="/docs/core-concepts/synthesis-ticks" title="Run a node" desc="One command to join the testnet mesh." />
                  <DropdownItem icon={Shield} href="/docs/core-concepts/consensus-staking" title="Stake & earn" desc="Lock WAVE and earn synthesis rewards." />
                </DropdownSection>
                <DropdownSection title="Learn">
                  <DropdownItem icon={Zap} href="/docs/core-concepts/consensus-staking" title="Tokenomics" desc="Metabolic burn, rewards, and issuance." />
                  <DropdownItem icon={Globe} href="/docs/getting-started/testnet" title="Testnet" desc="Live seed, API endpoints, and status." />
                  <DropdownItem icon={Box} href="/docs/" title="FAQ" desc="Common questions answered." />
                </DropdownSection>
              </DropdownGrid>
            </NavDropdown>

            <NavDropdown label="Build">
              <DropdownCta
                title="Developer docs"
                desc="Everything you need to build dApps, nodes, and bridges on Fluidic."
                href="/docs/"
              />
              <DropdownGrid>
                <DropdownSection title="Start building">
                  <DropdownItem icon={Terminal} href="/docs/getting-started/quickstart" title="Quickstart" desc="Install the SDK and submit your first shift." />
                  <DropdownItem icon={Layers} href="/docs/core-concepts/shifts" title="Architecture" desc="NTT, vector-clock DAG, and synthesis." />
                  <DropdownItem icon={Cpu} href="/docs/api-reference/typescript-sdk" title="SDK reference" desc="TypeScript client, keys, and swaps." />
                </DropdownSection>
                <DropdownSection title="References">
                  <DropdownItem icon={AppWindow} href="/dapp" title="Reference dApp" desc="Live example: wallet, faucet, and swap." />
                  <DropdownItem icon={Radio} href="/docs/api-reference/rest-api" title="API reference" desc="REST and WebSocket endpoints." />
                  <DropdownItem icon={Blocks} href="/docs/core-concepts/evm-compatibility" title="EVM bridge" desc="Execute raw Ethereum transactions." />
                  <DropdownItem icon={Terminal} href="https://github.com/Fluidic-Foundation/Fluidic-FVM" title="GitHub" desc="Open-source node runtime." />
                </DropdownSection>
              </DropdownGrid>
            </NavDropdown>

            <NavDropdown label="Network">
              <DropdownCta
                title="Network status"
                desc="Explore live blocks, shifts, validators, and quorum status."
                href="/explorer"
              />
              <DropdownGrid>
                <DropdownSection title="Explore">
                  <DropdownItem icon={Globe} href="/explorer" title="Explorer" desc="Search shifts, ticks, and validators." />
                  <DropdownItem icon={Network} href="/docs/getting-started/testnet" title="Testnet info" desc="Seed peer, RPC, and faucet." />
                  <DropdownItem icon={Shield} href="/docs/core-concepts/consensus-staking" title="Validators" desc="Staking, quorum, and certificates." />
                </DropdownSection>
                <DropdownSection title="Participate">
                  <DropdownItem icon={Cpu} href="/docs/core-concepts/synthesis-ticks" title="Synthesis nodes" desc="Run a node and produce certificates." />
                  <DropdownItem icon={Blocks} href="/docs/core-concepts/synthesis-ticks" title="Blocks / ticks" desc="How synthesis ticks become certificates." />
                  <DropdownItem icon={Zap} href="/docs/core-concepts/consensus-staking" title="Quorum" desc="BFT finalization mechanics." />
                </DropdownSection>
              </DropdownGrid>
            </NavDropdown>

            <NavDropdown label="Ecosystem" full>
              <DropdownGrid>
                <DropdownSection title="Resources">
                  <DropdownItem icon={Box} href="https://github.com/Fluidic-Foundation/Fluidic-FVM" title="Main repo" desc="Full project source and whitepaper." />
                  <DropdownItem icon={Cpu} href="https://github.com/Fluidic-Foundation/Fluidic-FVM" title="Node runtime" desc="Minimal repo for running a node." />
                  <DropdownItem icon={Terminal} href="https://github.com/Kolacjechutny/fluidic/tree/main/sdk/typescript" title="TypeScript SDK" desc="Sign shifts and build clients." />
                </DropdownSection>
                <DropdownSection title="Community">
                  <DropdownItem icon={Globe} href="/explorer" title="Testnet explorer" desc="Watch the mesh live." />
                  <DropdownItem icon={BookOpen} href="/docs/" title="Documentation" desc="User and developer guides." />
                  <DropdownItem icon={Zap} href="/whitepaper.pdf" title="Whitepaper" desc="Protocol specification and design." />
                </DropdownSection>
              </DropdownGrid>
            </NavDropdown>
          </div>
        </div>
        <div className="dock-right">
          <span className="status-indicator">Testnet Live</span>
          <Link href="/explorer" className="hidden sm:block">
            <button className="status-button">OPEN EXPLORER</button>
          </Link>
          <button
            className="mobile-menu-toggle"
            aria-label="Open menu"
            onClick={() => setMobileOpen(true)}
          >
            <Menu className="h-5 w-5" />
          </button>
        </div>
      </nav>

      {mobileOpen && (
        <div className="mobile-menu-overlay" onClick={() => setMobileOpen(false)}>
          <div className="mobile-menu" onClick={(e) => e.stopPropagation()}>
            <div className="mobile-menu-header">
              <span className="brand-name">Fluidic</span>
              <button aria-label="Close menu" onClick={() => setMobileOpen(false)}>
                <X className="h-5 w-5" />
              </button>
            </div>
            <div className="mobile-menu-links">
              <Link href="/docs/" onClick={() => setMobileOpen(false)}>Use Fluidic</Link>
              <Link href="/docs/" onClick={() => setMobileOpen(false)}>Build</Link>
              <Link href="/explorer" onClick={() => setMobileOpen(false)}>Network</Link>
              <Link href="/docs/" onClick={() => setMobileOpen(false)}>Ecosystem</Link>
              <Link href="/explorer" onClick={() => setMobileOpen(false)}>Explorer</Link>
            </div>
          </div>
        </div>
      )}

      <header className="hero-section">
        <div className="hero-coord">[WAVE_0X // SYNTH_01]</div>
        <h1 className="hero-title">
          Continuous-wave<br />
          <span className="text-primary glow-primary">state synthesis</span>
        </h1>
        <div className="hero-text-grid">
          <p>
            NTT-AGGREGATED COMMUTATIVE SHIFTS, VECTOR-CLOCK DAG ORDERING<br />
            FOR STATEFUL OPERATIONS, AND A METABOLIC BURN ENGINE.<br />
            NO BLOCKS. NO MEMPOOL. JUST SYNTHESIZED STATE.
          </p>
          <p>
            ANYONE CAN RUN A SYNTHESIS NODE, STAKE FLUIDIC, AND JOIN<br />
            THE PERMISSIONLESS MESH. SUB-SECOND FINALITY WITH BFT<br />
            CERTIFICATES AND A NATIVE EVM SANDBOX.
          </p>
        </div>
      </header>

      <main className="masonry-grid">
        {/* Pixel Wave Crypto Stream */}
        <div className="data-node pixel-wave-wrapper" style={{ gridColumn: 'span 12', padding: 0 }}>
          <div className="node-header" style={{ position: 'absolute', top: '24px', left: '24px', zIndex: 10 }}>
            <span className="node-title text-secondary">VALUE STREAM CONVERGENCE</span>
            <span className="node-coord">[WAVE_0X]</span>
          </div>

          <div className="wave-layer l3">
            {WAVE_PIXELS.map((_, i) => (
              <div key={`l3-${i}`} className="pixel-dot" style={{ animationDelay: `${i * 0.05}s` }} />
            ))}
          </div>
          <div className="wave-layer l2">
            {WAVE_PIXELS.map((_, i) => (
              <div key={`l2-${i}`} className="pixel-dot" style={{ animationDelay: `${i * 0.08}s` }} />
            ))}
          </div>
          <div className="wave-layer l1">
            {WAVE_PIXELS.map((_, i) => (
              <div key={`l1-${i}`} className="pixel-dot" style={{ animationDelay: `${i * 0.06}s` }} />
            ))}
          </div>

          <div className="flowing-coins">
            <div className="coin c-1">[WAVE]</div>
            <div className="coin c-2">[USDC]</div>
            <div className="coin c-3">[ETH]</div>
            <div className="coin c-4">[FLUID]</div>
            <div className="coin c-5">[BTC]</div>
            <div className="coin c-6">[SOL]</div>
          </div>
        </div>

        {/* Burn / economics node */}
        <div className="data-node node-burn">
          <div className="node-header">
            <span className="node-title">METABOLIC BURN ENGINE</span>
            <span className="node-coord">[ECON_01]</span>
          </div>
          <div className="node-body">
            <div className="burn-value">
              <span className="metrics-lg">0.04</span> <span className="metrics-unit">%/tick</span>
            </div>
            <div className="burn-chart">
              <div className="chart-bar"></div>
              <div className="chart-bar"></div>
              <div className="chart-bar"></div>
              <div className="chart-bar"></div>
              <div className="chart-bar"></div>
              <div className="chart-labels">
                Continuous<br />
                <span className="text-primary">burn-reward loop</span>
              </div>
            </div>
          </div>
        </div>

        <div className="data-node node-topology">
          <div className="node-header">
            <span className="node-title text-white">Wave-field topology</span>
            <span className="node-coord">[NODE_A]</span>
          </div>
          <div className="topology-visual">
            <svg width="100%" height="100%" style={{ position: 'absolute', top: 0, left: 0, zIndex: 1 }}>
              <defs>
                <linearGradient id="line-grad">
                  <stop offset="0%" stopColor="rgba(189, 244, 255, 0.1)" />
                  <stop offset="100%" stopColor="rgba(255, 255, 255, 0.3)" />
                </linearGradient>
              </defs>
              <line x1="25%" y1="45%" x2="75%" y2="75%" stroke="url(#line-grad)" strokeWidth="1" />
              <line x1="25%" y1="45%" x2="75%" y2="75%" stroke="var(--secondary)" strokeWidth="2" className="animated-stream" />
            </svg>
            <div className="topo-node n1" style={{ zIndex: 2 }}></div>
            <div className="topo-node n2" style={{ zIndex: 2 }}></div>
          </div>
          <p className="node-desc">
            Stateful shifts follow causal paths in a vector-clock DAG while commutative shifts merge in parallel through NTT windows.
          </p>
        </div>

        {/* Stats */}
        <div className="data-node" style={{ gridColumn: "span 3" }} id="platform">
          <div className="node-header">
            <span className="node-title">FINALITY</span>
            <span className="node-coord">[STAT_01]</span>
          </div>
          <div className="metrics-lg text-primary glow-primary mt-auto">&lt;1s</div>
          <p className="node-desc mt-4">Synthesis-tick finalization</p>
        </div>

        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title">THROUGHPUT</span>
            <span className="node-coord">[STAT_02]</span>
          </div>
          <div className="metrics-lg text-secondary mt-auto" style={{ fontSize: '48px' }}>100K+</div>
          <p className="node-desc mt-4">NTT ops / sec</p>
        </div>

        <div className="data-node node-ecosystem" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title">ECOSYSTEM</span>
            <span className="node-coord">[STAT_03]</span>
          </div>
          <div className="ecosystem-logos">
            <img src="/optimism.png" className="eco-logo l-1" alt="Optimism" />
            <img src="/polygon.webp" className="eco-logo l-2" alt="Polygon" />
            <img src="/arbitrum.png" className="eco-logo l-3" alt="Arbitrum" />
            <img src="/base.png" className="eco-logo l-4" alt="Base" />
          </div>
          <div className="metrics-lg text-accent mt-auto" style={{ fontSize: '48px' }}>2,000+</div>
          <p className="node-desc mt-4">EVM chains supported</p>
        </div>

        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title">DOMAINS</span>
            <span className="node-coord">[STAT_04]</span>
          </div>
          <div className="metrics-lg text-white mt-auto">∞</div>
          <p className="node-desc mt-4">Isolated concurrency scopes</p>
        </div>

        {/* Builders / Users */}
        <div className="data-node" style={{ gridColumn: "span 6" }} id="solutions">
          <div className="node-header">
            <span className="node-title text-primary">WHY BUILDERS CHOOSE FLUIDIC</span>
            <span className="node-coord">[BENE_01]</span>
          </div>
          <div className="validator-list mt-8">
            <div className="validator-item">
              <span className="text-white">Commutative throughput</span>
              <span className="val-status active">Batch-sum pool deltas via NTT</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Causal ordering</span>
              <span className="val-status active">Vector-clock DAG for stateful shifts</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Permissionless nodes</span>
              <span className="val-status active">One Docker command to validate</span>
            </div>
            <div className="validator-item">
              <span className="text-white">EVM sandbox</span>
              <span className="val-status active">Run raw Ethereum transactions</span>
            </div>
          </div>
        </div>

        <div className="data-node" style={{ gridColumn: "span 6" }}>
          <div className="node-header">
            <span className="node-title text-secondary">HOW USERS BENEFIT</span>
            <span className="node-coord">[BENE_02]</span>
          </div>
          <div className="validator-list mt-8">
            <div className="validator-item">
              <span className="text-white">Low fees</span>
              <span className="val-status active text-secondary">No block-space auction</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Fast finality</span>
              <span className="val-status active text-secondary">Certificates every tick</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Fair ordering</span>
              <span className="val-status active text-secondary">Causal, not extractable</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Open access</span>
              <span className="val-status active text-secondary">Run a node from GitHub</span>
            </div>
          </div>
        </div>

        {/* Industry Cards */}
        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title">DeFi</span>
            <span className="node-coord">[IND_01]</span>
          </div>
          <ul className="mt-8 space-y-4 node-desc text-sm">
            <li>› CLOBs and AMMs on one engine</li>
            <li>› Cross-domain composability</li>
            <li>› MEV-resistant causal ordering</li>
          </ul>
        </div>

        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title text-secondary">AI Agents</span>
            <span className="node-coord">[IND_02]</span>
          </div>
          <ul className="mt-8 space-y-4 node-desc text-sm">
            <li>› Per-agent state domains</li>
            <li>› Verifiable action logs</li>
            <li>› Pay-per-signal economics</li>
          </ul>
        </div>

        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title text-accent">Gaming</span>
            <span className="node-coord">[IND_03]</span>
          </div>
          <ul className="mt-8 space-y-4 node-desc text-sm">
            <li>› Tick-synchronized game state</li>
            <li>› Asset ownership without congestion</li>
            <li>› High-frequency player actions</li>
          </ul>
        </div>

        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title" style={{ color: '#f59e0b' }}>Institutions</span>
            <span className="node-coord">[IND_04]</span>
          </div>
          <ul className="mt-8 space-y-4 node-desc text-sm">
            <li>› Auditable Merkle certificates</li>
            <li>› Private compliance domains</li>
            <li>› Deterministic settlement</li>
          </ul>
        </div>

        {/* Code Block */}
        <div className="node-sandbox" id="developers" style={{ gridColumn: "span 6" }}>
          <div className="data-node" style={{ height: '100%' }}>
            <div className="node-header">
              <span className="node-title text-primary">START CODING</span>
              <span className="node-coord">[DEV_01]</span>
            </div>
            <div className="code-block mt-6 flex-1">
              <pre>
{`const client = new FluidicClient({ apiUrl });
const wallet = FluidicKeypair.generate();
await client.register(wallet.publicKeyHex);

const { poolInHash } = await submitSwap(client, {
  signer: wallet,
  direction: "WAVE_TO_USDC",
  amount: 1_000_000n,
});
console.log(poolInHash);`}
              </pre>
            </div>
            <div className="start-coding-footer" style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', marginTop: '16px' }}>
              <p className="node-desc" style={{ margin: 0, maxWidth: '70%' }}>
                Register an account, sign a swap, and watch it synthesize in the explorer.
              </p>
              <Link href="/docs/" className="docs-btn">
                READ DOCS
                <span className="arrow">→</span>
              </Link>
            </div>
          </div>
        </div>

        {/* Start Earning / Building */}
        <div className="data-node" style={{ gridColumn: "span 6" }}>
          <div className="node-header">
            <span className="node-title text-secondary">START BUILDING & EARNING</span>
            <span className="node-coord">[DEV_02]</span>
          </div>
          <div className="mt-8 space-y-8">
            <div>
              <h3 className="text-white text-xl font-light mb-2" style={{ fontFamily: 'var(--font-header)' }}>Start building</h3>
              <p className="node-desc text-sm">Spin up a node, register an account, and submit your first stateful or commutative shift in minutes.</p>
            </div>
            <div>
              <h3 className="text-white text-xl font-light mb-2" style={{ fontFamily: 'var(--font-header)' }}>Start earning</h3>
              <p className="node-desc text-sm">Run a synthesis node, stake WAVE, and earn a share of metabolic burn rewards every tick.</p>
            </div>
            <div>
              <h3 className="text-white text-xl font-light mb-2" style={{ fontFamily: 'var(--font-header)' }}>Start connecting</h3>
              <p className="node-desc text-sm">Follow the testnet explorer, read the architecture docs, and contribute to the open-source mesh.</p>
            </div>
          </div>
        </div>

        {/* Reference DApp */}
        <div className="data-node" style={{ gridColumn: "span 12" }}>
          <div className="node-header">
            <span className="node-title text-accent">REFERENCE DAPP</span>
            <span className="node-coord">[DAPP_01]</span>
          </div>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-6 mt-6">
            <p className="node-desc text-sm max-w-2xl">
              A minimal, live example for developers: create a wallet, drip 1,000 WAVE + 1,000 USDC from the faucet, and swap continuously on the testnet mesh. Open-source and ready to fork.
            </p>
            <Link href="/dapp" className="docs-btn shrink-0">
              OPEN REFERENCE DAPP
              <ArrowRight className="h-3 w-3" />
            </Link>
          </div>
        </div>

        {/* Integration Dock */}
        <div className="integration-dock" id="community">
          <div className="integration-header">
            <div className="window-dots">
              <div className="dot red"></div>
              <div className="dot yellow"></div>
              <div className="dot green"></div>
            </div>
            <span className="file-path">~/fluidic/run-node.sh</span>
          </div>
          <div className="integration-code">
            <pre>
              <span className="code-comment" style={{ color: 'rgba(255,255,255,0.4)' }}># One command joins the testnet mesh.</span><br /><br />
              <span className="code-keyword">docker</span> run -d --name fluidic-node <span className="code-class">\</span><br />
              {'  '}-p 8080:8080 -p 7000:7000 <span className="code-class">\</span><br />
              {'  '}-e OSCILLATOR_ID=node-1 <span className="code-class">\</span><br />
              {'  '}-e PEERS=<span className="code-string">&quot;34.56.159.76:7000&quot;</span> <span className="code-class">\</span><br />
              {'  '}ghcr.io/Fluidic-Foundation/Fluidic-FVM:latest<br /><br />
              <span className="code-keyword">curl</span> http://localhost:8080/api/health
            </pre>
          </div>
        </div>
      </main>

      <footer className="footer-dock">
        <div className="footer-left text-primary">
          [WAVE_0X // SYNTH_01] © {new Date().getFullYear()} FLUIDIC FOUNDATION // NULL_DOMAIN
        </div>
        <div className="footer-links">
          <Link href="/docs/">Documentation</Link>
          <Link href="/explorer">Explorer</Link>
          <a href="https://github.com/Fluidic-Foundation/">GitHub</a>
          <Link href="/docs/getting-started/testnet">Testnet</Link>
          <a href="https://github.com/Fluidic-Foundation/Fluidic-FVM">Source</a>
        </div>
        <div className="footer-right text-white" style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <div className="brand-container">
            <span className="brand-name" style={{ color: 'var(--white)' }}>FLUIDIC</span>
            <img src="/fluidic-logo-new.png" className="brand-logo" alt="Fluidic Logo" />
          </div>
        </div>
      </footer>
    </div>
  );
}

function NavDropdown({ label, children, full }: { label: string; children: React.ReactNode; full?: boolean }) {
  return (
    <div className="nav-dropdown">
      <button className="nav-dropdown-trigger">
        {label} <ChevronDown className="h-3 w-3" />
      </button>
      <div className={`nav-dropdown-menu ${full ? 'nav-dropdown-full' : ''}`}>
        {children}
      </div>
    </div>
  );
}

function DropdownCta({ title, desc, href }: { title: string; desc: string; href: string }) {
  return (
    <div className="nav-dropdown-cta">
      <div>
        <h4>{title}</h4>
        <p>{desc}</p>
      </div>
      <Link href={href} className="docs-btn">
        Explore <ArrowRight className="h-3 w-3" />
      </Link>
    </div>
  );
}

function DropdownGrid({ children }: { children: React.ReactNode }) {
  return <div className="nav-dropdown-grid">{children}</div>;
}

function DropdownSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="nav-dropdown-section">
      <div className="nav-dropdown-section-title">{title}</div>
      {children}
    </div>
  );
}

function DropdownItem({
  icon: Icon,
  href,
  title,
  desc,
}: {
  icon: React.ElementType;
  href: string;
  title: string;
  desc: string;
}) {
  const isExternal = href.startsWith('http');
  return (
    <Link href={href} target={isExternal ? '_blank' : undefined} rel={isExternal ? 'noopener noreferrer' : undefined} className="nav-dropdown-item">
      <div className="nav-dropdown-item-icon">
        <Icon className="h-4 w-4" />
      </div>
      <div className="nav-dropdown-item-text">
        <h5>{title}</h5>
        <p>{desc}</p>
      </div>
    </Link>
  );
}
