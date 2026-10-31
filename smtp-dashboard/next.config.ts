import { resolve } from "node:path";
import type { NextConfig } from "next";

function getAPIOrigin(): string {
  const configuredOrigin = process.env.API_URL?.trim();
  if (!configuredOrigin) {
    if (process.env.NODE_ENV === "development") {
      return "http://127.0.0.1:8080";
    }
    throw new Error(
      "API_URL must be configured as an absolute HTTP(S) origin for non-development builds.",
    );
  }

  let url: URL;
  try {
    url = new URL(configuredOrigin);
  } catch {
    throw new Error("API_URL must be an absolute HTTP(S) origin.");
  }

  if (
    (url.protocol !== "http:" && url.protocol !== "https:") ||
    !url.host ||
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.search ||
    url.hash
  ) {
    throw new Error(
      "API_URL must be an HTTP(S) origin without credentials, path, query, or fragment.",
    );
  }

  return url.origin;
}

const nextConfig: NextConfig = {
  cacheComponents: true,
  partialPrefetching: true,
  async rewrites() {
    const apiOrigin = getAPIOrigin();
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
