# Fluidic Dev Reference DApp

A minimal React + TypeScript reference application for interacting with a Fluidic mesh node. It demonstrates the core client flows:

- Creating an Ed25519 wallet in the browser.
- Registering the wallet with the node (faucet drip).
- Querying WAVE/USDC balances.
- Submitting commutative WAVE/USDC swaps through the continuous-state mesh.

This dApp is intentionally simple — it is meant as a starting point for builders integrating the [`@fluidic-foundation/sdk`](node_modules/@fluidic-foundation/sdk).

## Live version

The reference dApp is deployed at:

```
https://testnet.fluidic.foundation/dapp
```

It points to the public testnet API by default:

```
https://api.testnet.fluidic.foundation
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_FLUIDIC_API` | `https://api.testnet.fluidic.foundation` | Base URL of the Fluidic node HTTP/WebSocket API |

Create a `.env` file in `dapp/` to point at a local node:

```bash
VITE_FLUIDIC_API=http://localhost:8080
```

## Building for production

```bash
npm run build
```

The static output is written to `dist/` and can be served by any static host (e.g., Vercel, Cloudflare Pages, or a GCS bucket).

## Project structure

```
dapp/
├── src/
│   ├── App.tsx              # Main UI: wallet, faucet, swap, balances
│   ├── lib/
│   │   ├── fluidicClient.ts # SDK wrapper + localStorage wallet helpers
│   │   └── tokens.ts        # Token metadata
│   └── index.css            # Tailwind theme / custom properties
├── index.html
├── package.json
├── tailwind.config.js
└── vite.config.ts
```

## Wallet storage

The dApp stores the generated wallet in `localStorage` under `fluidic:dev-wallet`. This is fine for a testnet reference but should be replaced with a proper signer (e.g., browser extension, walletconnect, or a secure enclave) for production use.

## License

MIT OR Apache-2.0 — same as the Fluidic node.
