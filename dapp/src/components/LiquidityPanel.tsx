interface LiquidityPanelProps {
  history: Array<{ volume: number; price: number }>;
}

export function LiquidityPanel({ history }: LiquidityPanelProps) {
  const maxVolume = Math.max(...history.map((h) => h.volume), 1);

  return (
    <div className="bg-fluidic-card border border-fluidic-border rounded-xl p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold">Liquidity Depth</h2>
        <span className="text-xs text-fluidic-dim font-mono">REAL-TIME</span>
      </div>

      <div className="flex items-end gap-1 h-40 mb-4">
        {history.map((tick, i) => {
          const h = (tick.volume / maxVolume) * 100;
          return (
            <div
              key={i}
              className="flex-1 rounded-sm bg-fluidic-accent/40 hover:bg-fluidic-accent transition-colors"
              style={{ height: `${Math.max(4, h)}%` }}
              title={`${tick.volume.toFixed(0)} @ $${tick.price.toFixed(4)}`}
            />
          );
        })}
        {history.length === 0 && (
          <div className="w-full h-full flex items-center justify-center text-xs text-fluidic-dim">
            Waiting for live ticks…
          </div>
        )}
      </div>

      <div className="flex justify-between text-sm text-fluidic-muted">
        <span>Data points</span>
        <span className="text-white font-mono">{history.length}</span>
      </div>
    </div>
  );
}
