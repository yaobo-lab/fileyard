import createNextIntlPlugin from "next-intl/plugin"

const basePath = process.env.NEXT_PUBLIC_BASE_PATH ?? ""
const assetPrefix = process.env.NEXT_PUBLIC_ASSET_PREFIX ?? ""

/** @type {import('next').NextConfig} */
const nextConfig = {
  basePath,
  assetPrefix,
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
    // Only redirect "/" → basePath when a basePath is configured.
    return basePath
      ? [
          {
            source: "/",
            destination: basePath,
            permanent: false,
            basePath: false,
          },
        ]
      : []
  },
}

const withNextIntl = createNextIntlPlugin()

export default withNextIntl(nextConfig)
