const appUrl = process.env.NEXT_PUBLIC_APP_URL ?? "https://storageui.dev"

export const siteConfig = {
  name: "Storage UI",
  url: appUrl,
  websiteUrl: "https://storageui.dev",
  githubUrl: "https://github.com/hahahumble/storageui",
  ogImage: `${appUrl}/opengraph-image.png`,
  description:
    "Open-source file browser for S3, R2, and other storage backends. Browse, preview, search, and manage files in a modern, self-hosted web interface.",
}

export const META_THEME_COLORS = {
  light: "#ffffff",
  dark: "#09090b",
}
