# Fluidic Next.js Starter

A minimal Next.js app that connects to the public Fluidic testnet and submits a signed swap.

## Setup

```bash
npx degit Fluidic-Foundation/fluidic/starters/nextjs my-fluidic-app
cd my-fluidic-app
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) and click:
1. **Generate wallet**
2. **Fund** (testnet faucet)
3. **Swap** (WAVE → USDC)

The app uses `@fluidic-foundation/sdk` and talks to `https://api.testnet.fluidic.foundation`.
