const BANDS = [
  { app: 'DEX Pool', start: 0, width: 18, color: '#00E5C9' },
  { app: 'Agent Swarm', start: 18, width: 25, color: '#3B82F6' },
  { app: 'RWA Feed', start: 43, width: 16, color: '#10B981' },
];

export function SpectrumBand() {
  return (
    <div className="bg-fluidic-card border border-fluidic-border rounded-xl p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold">Spectrum Allocation</h2>
        <span className="text-xs text-fluidic-dim font-mono">BANDWIDTH LEASES</span>
      </div>

      <div className="relative h-12 bg-fluidic-bg rounded-lg overflow-hidden border border-fluidic-border">
        {BANDS.map((band) => (
          <div
            key={band.app}
            className="absolute top-0 h-full flex items-center justify-center text-xs font-semibold text-white"
            style={{
              left: `${band.start}%`,
              width: `${band.width}%`,
              backgroundColor: band.color,
              opacity: 0.35,
              border: `1px solid ${band.color}`,
            }}
          >
            {band.width > 10 && <span className="drop-shadow">{band.app}</span>}
          </div>
        ))}
      </div>

      <div className="mt-4 grid grid-cols-3 gap-2">
        {BANDS.map((band) => (
          <div key={band.app} className="flex items-center gap-2 text-xs">
            <div className="w-3 h-3 rounded-sm" style={{ backgroundColor: band.color }} />
            <span className="text-fluidic-muted">{band.app}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
