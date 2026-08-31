import createNextIntlPlugin from "next-intl/plugin"

const basePath = process.env.NEXT_PUBLIC_BASE_PATH ?? ""
const assetPrefix = process.env.NEXT_PUBLIC_ASSET_PREFIX ?? ""

/** @type {import('next').NextConfig} */
const nextConfig = {
  basePath,
  assetPrefix,
  allowedDevOrigins: [
    '172.21.96.1:8080',
    '172.21.96.1:8081',
    '172.21.96.1:3000',
    '172.21.96.1',
    '192.168.3.42:8080',
    '192.168.3.42:8081',
    '192.168.3.42:3000',
    '192.168.3.42',
    'localhost:8080',
    'localhost:8081',
    'localhost:3000',
    'localhost',
  ],
  devIndicators: false,
  reactCompiler: true,
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "avatars.githubusercontent.com",
      },
      {
        protocol: "https",
        hostname: "images.unsplash.com",
      },
      {
        protocol: "https",
        hostname: "avatar.vercel.sh",
      },
    ],
  },
  turbopack: {
    root: import.meta.dirname,
    resolveAlias: {
      "@dukelib/sheets-wasm/duke_sheets_wasm_bg.wasm":
        "./lib/turbopack-duke-sheets-wasm-url.ts",
    },
  },
  experimental: {
    turbopackFileSystemCacheForDev: true,
  },
  webpack: (config) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
    }

    config.module.rules.unshift({
      resourceQuery: /url/,
      test: /\.wasm$/,
      type: "asset/resource",
    })

    return config
  },
  redirects() {
    return []
  },
}

const withNextIntl = createNextIntlPlugin()

export default withNextIntl(nextConfig)
