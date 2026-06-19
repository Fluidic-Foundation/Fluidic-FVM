import type { AccountInfo } from '../lib/fluidicClient';

interface HeaderProps {
  connected: boolean;
  account: AccountInfo | null;
  onCreateAccount: () => void;
}

export function Header({ connected, account, onCreateAccount }: HeaderProps) {
  return (
    <header className="flex items-center justify-between px-6 py-4 border-b border-fluidic-border bg-fluidic-bg">
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 overflow-hidden">
          <img
            src="/fluidic-logo.png"
            alt="Fluidic"
            className="h-full w-full object-contain"
          />
        </div>
        <div>
          <h1 className="text-lg font-bold tracking-tight">Fluidic</h1>
          <p className="text-xs text-fluidic-dim font-mono">CONTINUOUS-STATE DEX</p>
        </div>
      </div>
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2 text-xs">
          <span className={`w-2 h-2 rounded-full ${connected ? 'bg-green-500' : 'bg-red-500'}`} />
          <span className="text-fluidic-muted">{connected ? 'LIVE' : 'OFFLINE'}</span>
        </div>
        {account?.accountId ? (
          <div className="text-xs font-mono text-fluidic-accent">
            {account.accountId.slice(0, 8)}…{account.accountId.slice(-6)}
          </div>
        ) : (
          <button
            onClick={onCreateAccount}
            className="px-3 py-1.5 text-xs font-semibold bg-fluidic-accent text-fluidic-bg rounded-lg hover:opacity-90"
          >
            Create Account
          </button>
        )}
      </div>
    </header>
  );
}
