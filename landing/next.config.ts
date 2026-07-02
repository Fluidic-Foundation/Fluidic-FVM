import type { NextConfig } from "next";
import path from "path";

const nodeUrl = process.env.FLUIDIC_NODE_URL || "http://localhost:8080";

const nextConfig: NextConfig = {
  // NOTE: Static export is disabled on Railway so /api/* can proxy to the node.
  // Run `npm run build` locally if you need a static export for other hosts.
  output: process.env.STATIC_EXPORT === "true" ? "export" : undefined,
  distDir: "dist",
  trailingSlash: true,
  turbopack: {
    root: path.resolve(__dirname),
  },
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: `${nodeUrl}/api/:path*`,
      },
      {
        source: "/rpc",
        destination: `${nodeUrl}/rpc`,
      },
    ];
  },
};

export default nextConfig;
