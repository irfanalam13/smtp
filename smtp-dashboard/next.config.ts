import { resolve } from "node:path";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  cacheComponents: true,
  partialPrefetching: true,
  async rewrites() {
    const apiOrigin = (
      process.env.API_URL ??
      process.env.NEXT_PUBLIC_API_URL ??
      "http://127.0.0.1:8080"
    ).replace(/\/+$/, "");
    return [
      {
        source: "/api/backend/:path*",
        destination: `${apiOrigin}/:path*`,
      },
    ];
  },
  turbopack: {
    root: resolve(__dirname, "../.."),
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
