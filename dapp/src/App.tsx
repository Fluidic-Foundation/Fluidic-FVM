import { useEffect, useState } from 'react';
import { Header } from './components/Header';
import { Metrics } from './components/Metrics';
import { SwapCard } from './components/SwapCard';
import { LiquidityPanel } from './components/LiquidityPanel';
import { SpectrumBand } from './components/SpectrumBand';
import { createFluidicClient, type FluidicState, type AccountInfo } from './lib/fluidicClient';

const client = createFluidicClient();

function App() {
  const [state, setState] = useState<FluidicState>(client.state);
  const [account, setAccount] = useState<AccountInfo | null>(client.account);
  const [connected, setConnected] = useState(false);
  const [history, setHistory] = useState<Array<{ volume: number; price: number }>>([]);

  useEffect(() => {
    client.connect();
    const unsubscribe = client.subscribe((newState) => {
      setState(newState);
      setConnected(client.isConnected);
      setAccount(client.account);
      setHistory((prev) => {
        const next = [...prev, { volume: Number(newState.throughput), price: newState.price }];
        return next.slice(-60);
      });
    });

    const interval = setInterval(() => setConnected(client.isConnected), 1000);

    return () => {
      unsubscribe();
      clearInterval(interval);
      client.disconnect();
    };
  }, []);

  return (
    <div className="min-h-screen bg-fluidic-bg text-white">
      <Header
        connected={connected}
        account={account}
        onCreateAccount={() => client.createAccount().then(setAccount)}
      />
      <main className="max-w-6xl mx-auto px-6 py-8 space-y-6">
        <Metrics state={state} />

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <div className="lg:col-span-1">
            <SwapCard
              state={state}
              onSwap={(from, to, amount) => client.swap(from, to, amount)}
              onGetShiftStatus={(hash) => client.getShiftStatus(hash)}
              onGetBalance={(id) => client.getBalance(id)}
              account={account}
            />
          </div>
          <div className="lg:col-span-2 space-y-6">
            <LiquidityPanel history={history} />
            <SpectrumBand />
          </div>
        </div>

        <footer className="pt-8 text-center text-xs text-fluidic-dim font-mono">
          FLUIDIC CONTINUOUS-STATE DEX — CONNECTED TO LIVE MESH NODE
        </footer>
      </main>
    </div>
  );
}

export default App;
