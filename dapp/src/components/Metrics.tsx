import type { FluidicState } from '../lib/fluidicClient';

interface MetricsProps {
  state: FluidicState;
}

export function Metrics({ state }: MetricsProps) {
  const wave = Number(state.wave_reserve) / 1e12;
  const usdc = Number(state.usdc_reserve) / 1e12;

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
      <Metric label="Price" value={`$${state.price.toFixed(4)}`} sub="WAVE/USDC" />
      <Metric label="Pool WAVE" value={`${wave.toFixed(1)}`} sub="reserve" />
      <Metric label="Pool USDC" value={`${usdc.toFixed(1)}`} sub="reserve" />
      <Metric label="Latency" value={`${state.latency_ms.toFixed(1)} ms`} sub="p99" />
    </div>
  );
}

function Metric({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div className="bg-fluidic-card border border-fluidic-border rounded-xl p-4">
      <p className="text-xs text-fluidic-dim font-mono uppercase tracking-wider mb-1">{label}</p>
      <p className="text-2xl font-bold text-white">{value}</p>
      <p className="text-xs text-fluidic-muted mt-1">{sub}</p>
    </div>
  );
}
