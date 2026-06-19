/**
 * Fluidic Testnet Faucet
 *
 * Distributes test WAVE/USDC tokens to registered accounts by calling the
 * mesh_node registration endpoint, which already seeds derived token accounts.
 *
 * Endpoints:
 *   POST /faucet  { publicKeyHex: string }
 *   GET  /health
 */

const express = require("express");
const rateLimit = require("express-rate-limit");

const NODE_API = process.env.NODE_API_URL || "http://mesh-node:8080";
const PORT = process.env.PORT || 3000;

const app = express();
app.use(express.json());

const limiter = rateLimit({
  windowMs: 60 * 60 * 1000, // 1 hour
  max: 10,
  standardHeaders: true,
  legacyHeaders: false,
  keyGenerator: (req) => req.body?.publicKeyHex || req.ip,
  handler: (req, res) => {
    res.status(429).json({ error: "Rate limit exceeded. One drip per hour." });
  },
});

app.post("/faucet", limiter, async (req, res) => {
  const { publicKeyHex } = req.body;

  if (!publicKeyHex || typeof publicKeyHex !== "string") {
    return res.status(400).json({ error: "publicKeyHex is required" });
  }

  if (!/^[0-9a-fA-F]{64}$/.test(publicKeyHex)) {
    return res.status(400).json({ error: "publicKeyHex must be 64 hex chars (Ed25519)" });
  }

  try {
    const upstream = await fetch(`${NODE_API}/api/account/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ public_key_hex: publicKeyHex }),
    });

    if (!upstream.ok) {
      const text = await upstream.text();
      return res.status(upstream.status).json({ error: text });
    }

    const data = await upstream.json();
    return res.json({
      status: "dripped",
      account_id: data.account_id,
      wave_account: data.wave_account,
      usdc_account: data.usdc_account,
    });
  } catch (err) {
    console.error("faucet error:", err);
    return res.status(502).json({ error: "Unable to reach Fluidic node" });
  }
});

app.get("/health", (_req, res) => {
  res.json({ status: "ok", node: NODE_API });
});

app.listen(PORT, () => {
  console.log(`Fluidic faucet listening on port ${PORT}`);
});
