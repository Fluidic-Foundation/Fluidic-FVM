export default function FluidicPage() {
  const WAVE_PIXELS = Array.from({ length: 80 });

  return (
    <div className="infinite-matrix-canvas">
      {/* Background Ambient Glows */}
      <div className="ambient-glow glow-1"></div>
      <div className="ambient-glow glow-2"></div>
      <div className="ambient-glow glow-3"></div>

      <div className="vertical-label">ARCHITECTURAL INVERSIONS</div>
      
      <nav className="floating-dock">
        <div className="dock-left">
          <div className="brand-container">
            <img src="/fluidic-logo-new.png" className="brand-logo" alt="Fluidic Logo" />
            <span className="brand-name">Fluidic</span>
          </div>
          <div className="dock-links">
            <a href="#platform">Platform</a>
            <a href="#solutions">Solutions</a>
            <a href="#developers">Developers</a>
            <a href="#community">Community</a>
          </div>
        </div>
        <div className="dock-right">
          <span className="status-indicator">Network Peer Status</span>
          <button className="status-button">SYS_PORTAL_UP</button>
        </div>
      </nav>

      <header className="hero-section">
        <div className="hero-coord">[42.08 // 09.11]</div>
        <h1 className="hero-title">
          Build full-stack<br />
          <span className="text-primary glow-primary">async finance</span>
        </h1>
        <div className="hero-text-grid">
          <p>
            SUB-MILLISECOND FINALITY, PARALLEL STATE ACTORS,<br />
            AND A NATIVE EVM SANDBOX. FLUIDIC REPLACES THE<br />
            SEQUENTIAL BLOCK WITH AN ASYNCHRONOUS WAVE-FIELD.
          </p>
          <p>
            THE BLOCKLESS EXECUTION MESH IS NOW PUBLIC.<br />
            WAVE CONVERGENCE DETECTED. LATENCY APPROACHING<br />
            ABSOLUTE ZERO BOUNDARY.
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
            <div className="coin c-1">[ETH]</div>
            <div className="coin c-2">[USDC]</div>
            <div className="coin c-3">[BTC]</div>
            <div className="coin c-4">[FLUID]</div>
            <div className="coin c-5">[SOL]</div>
            <div className="coin c-6">[APT]</div>
          </div>
        </div>

        {/* Top visual nodes inspired by the design */}
        <div className="data-node node-burn">
          <div className="node-header">
            <span className="node-title">METABOLIC BURN REALLOCATION</span>
            <span className="node-coord">[METRIC_02]</span>
          </div>
          <div className="node-body">
            <div className="burn-value">
              <span className="metrics-lg">4.82</span> <span className="metrics-unit">Gwei</span>
            </div>
            <div className="burn-chart">
              <div className="chart-bar"></div>
              <div className="chart-bar"></div>
              <div className="chart-bar"></div>
              <div className="chart-bar"></div>
              <div className="chart-bar"></div>
              <div className="chart-labels">
                Optimal<br />
                <span className="text-primary">+0.04% Shift/s</span>
              </div>
            </div>
          </div>
        </div>

        <div className="data-node node-topology">
          <div className="node-header">
            <span className="node-title text-white">Asynchronous Topology</span>
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
            Non-blocking communication pathways mapping state vectors in n-dimensional continuous space.
          </p>
        </div>

        {/* Stats Section Converted to Nodes */}
        <div className="data-node" style={{ gridColumn: "span 3" }} id="platform">
          <div className="node-header">
            <span className="node-title">FINALITY</span>
            <span className="node-coord">[STAT_01]</span>
          </div>
          <div className="metrics-lg text-primary glow-primary mt-auto">&lt;1ms</div>
          <p className="node-desc mt-4">Speed without trade-offs</p>
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
            <span className="node-title">SCALE</span>
            <span className="node-coord">[STAT_04]</span>
          </div>
          <div className="metrics-lg text-white mt-auto">∞</div>
          <p className="node-desc mt-4">Parallel actors</p>
        </div>

        {/* Manifesto & Solutions converted to Nodes */}
        <div className="data-node" style={{ gridColumn: "span 6" }} id="solutions">
          <div className="node-header">
            <span className="node-title text-primary">WHY BUILDERS CHOOSE FLUIDIC</span>
            <span className="node-coord">[BENE_01]</span>
          </div>
          <div className="validator-list mt-8">
            <div className="validator-item">
              <span className="text-white">Modular Architecture</span>
              <span className="val-status active">Build faster with modular state actors</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Parallel Execution</span>
              <span className="val-status active">Scale on contention-free execution</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Sub-ms Finality</span>
              <span className="val-status active">Deliver familiar EVM experiences</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Verifiable Ordering</span>
              <span className="val-status active">Bring real-world assets on-chain</span>
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
              <span className="text-white">Sovereignty</span>
              <span className="val-status active text-secondary">Real ownership over assets & data</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Economics</span>
              <span className="val-status active text-secondary">Value created becomes value earned</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Reliability</span>
              <span className="val-status active text-secondary">Confidence that apps behave as expected</span>
            </div>
            <div className="validator-item">
              <span className="text-white">Access</span>
              <span className="val-status active text-secondary">Experiences that are open and rewarding</span>
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
            <li>› One-second swaps with sub-ms finality</li>
            <li>› Programmable liquidity</li>
            <li>› Cross-chain interoperability</li>
          </ul>
        </div>

        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title text-secondary">AI Agents</span>
            <span className="node-coord">[IND_02]</span>
          </div>
          <ul className="mt-8 space-y-4 node-desc text-sm">
            <li>› Autonomous actors with private state</li>
            <li>› Verifiable model outputs</li>
            <li>› Trustless agent coordination</li>
          </ul>
        </div>

        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title text-accent">Gaming</span>
            <span className="node-coord">[IND_03]</span>
          </div>
          <ul className="mt-8 space-y-4 node-desc text-sm">
            <li>› Real ownership of in-game assets</li>
            <li>› High-performance scalability</li>
            <li>› Seamless player experiences</li>
          </ul>
        </div>

        <div className="data-node" style={{ gridColumn: "span 3" }}>
          <div className="node-header">
            <span className="node-title" style={{ color: '#f59e0b' }}>Institutions</span>
            <span className="node-coord">[IND_04]</span>
          </div>
          <ul className="mt-8 space-y-4 node-desc text-sm">
            <li>› Secure, verifiable workflows</li>
            <li>› Scalable data and asset management</li>
            <li>› Transparency without compromise</li>
          </ul>
        </div>

        {/* Code Block / Start Coding */}
        <div className="node-sandbox" id="developers" style={{ gridColumn: "span 6" }}>
          <div className="data-node" style={{ height: '100%' }}>
            <div className="node-header">
              <span className="node-title text-primary">START CODING</span>
              <span className="node-coord">[DEV_01]</span>
            </div>
            <div className="code-block mt-6 flex-1">
              <pre>
                function synthesize( bytes32 _waveId, uint256 _amplitude ) external returns (bool) {'{'}{'\n'}
                {'  '}require(_amplitude &gt; 0); StateMap[_waveId] = _amplitude; emit Synthesis(_waveId); return true; {'}'}
              </pre>
            </div>
            <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', marginTop: '16px' }}>
              <p className="node-desc" style={{ margin: 0, maxWidth: '70%' }}>
                Use the TypeScript SDK or raw REST API to submit shifts, query state, and bridge EVM transactions.
              </p>
              <a href="#" className="docs-btn">
                READ DOCS
                <span className="arrow">→</span>
              </a>
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
              <p className="node-desc text-sm">Spin up a node, register an account, and deploy your first state actor in minutes.</p>
            </div>
            <div>
              <h3 className="text-white text-xl font-light mb-2" style={{ fontFamily: 'var(--font-header)' }}>Start earning</h3>
              <p className="node-desc text-sm">Stake FLUIDIC, run a synthesis node, and earn a share of metabolic burn rewards.</p>
            </div>
            <div>
              <h3 className="text-white text-xl font-light mb-2" style={{ fontFamily: 'var(--font-header)' }}>Start connecting</h3>
              <p className="node-desc text-sm">Join the Discord, follow the testnet explorer, and contribute to the open-source mesh.</p>
            </div>
          </div>
        </div>
        
        {/* Integration Dock Code Block (Community) */}
        <div className="integration-dock" id="community">
          <div className="integration-header">
            <div className="window-dots">
              <div className="dot red"></div>
              <div className="dot yellow"></div>
              <div className="dot green"></div>
            </div>
            <span className="file-path">~/fluidic/stay-in-loop.ts</span>
          </div>
          <div className="integration-code">
            <pre>
              <span className="code-comment" style={{ color: 'rgba(255,255,255,0.4)' }}>// Events, releases, operator guides, and SDK updates — all in one place.</span><br /><br />
              <span className="code-keyword">import</span> {'{'} Community {'}'} <span className="code-keyword">from</span> <span className="code-string">&apos;@fluidic/network&apos;</span>;<br /><br />
              <span className="code-keyword">const</span> subscriber = <span className="code-keyword">await</span> Community.<span className="code-method">join</span>({'{'}<br />
              {'  '}email: <span className="code-string">&apos;your@email.com&apos;</span>,<br />
              {'  '}interests: [<span className="code-string">&apos;Testnet explorer&apos;</span>, <span className="code-string">&apos;Synthesis nodes&apos;</span>, <span className="code-string">&apos;SDK v0.1&apos;</span>]<br />
              {'}'});<br /><br />
              <span className="code-keyword">await</span> subscriber.<span className="code-method">subscribe</span>();
            </pre>
          </div>
        </div>
      </main>

      <footer className="footer-dock">
        <div className="footer-left text-primary">
          [42.08 // 09.11] © {new Date().getFullYear()} FLUIDIC FOUNDATION // NULL_DOMAIN
        </div>
        <div className="footer-links">
          <a href="#">Overview</a>
          <a href="#">Technology</a>
          <a href="#">Documentation</a>
          <a href="#">GitHub</a>
          <a href="#">Discord</a>
          <a href="#">X</a>
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
