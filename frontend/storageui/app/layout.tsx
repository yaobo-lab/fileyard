import type { Metadata } from "next"
import Script from "next/script"
import { Analytics } from "@vercel/analytics/next"
import { NextIntlClientProvider } from "next-intl"
import { getLocale, getTranslations } from "next-intl/server"
import { NuqsAdapter } from "nuqs/adapters/next/app"

import { withUiBasePath } from "@/lib/config/base-path"
import { fontVariables } from "@/lib/config/fonts"
import { META_THEME_COLORS, siteConfig } from "@/lib/config/site"
import { cn } from "@/lib/utils"
import { Toaster } from "@/components/ui/sonner"
import { TooltipProvider } from "@/components/ui/tooltip"
import { ActiveThemeProvider } from "@/components/providers/active-theme"
import { TailwindIndicator } from "@/components/providers/tailwind-indicator"
import { ThemeFavicon } from "@/components/providers/theme-favicon"
import { ThemeProvider } from "@/components/providers/theme-provider"

import "@/app/globals.css"

const LIGHT_ICON_URL = withUiBasePath("/icon.svg")
const DARK_ICON_URL = withUiBasePath("/icon-dark.svg")
const PNG_ICON_URL = withUiBasePath("/icon.png")

const OG_LOCALES: Record<string, string> = { en: "en_US", zh: "zh_CN" }

export async function generateMetadata(): Promise<Metadata> {
  const t = await getTranslations("Metadata")
  const locale = await getLocale()
  const description = t("description")
  const appUrl = process.env.NEXT_PUBLIC_APP_URL ?? siteConfig.url
  const ogImageUrl = `${appUrl}/opengraph-image.png`

  return {
    title: {
      default: siteConfig.name,
      template: `%s - ${siteConfig.name}`,
    },
    metadataBase: new URL(appUrl),
    description,
    keywords: ["Next.js", "React", "Tailwind CSS", "Documents", "Components"],
    authors: [
      {
        name: siteConfig.name,
        url: siteConfig.url,
      },
    ],
    creator: siteConfig.name,
    openGraph: {
      type: "website",
      locale: OG_LOCALES[locale] ?? OG_LOCALES.en,
      url: appUrl,
      title: siteConfig.name,
      description,
      siteName: siteConfig.name,
      images: [
        {
          url: ogImageUrl,
          width: 1200,
          height: 630,
          alt: siteConfig.name,
        },
      ],
    },
    twitter: {
      card: "summary_large_image",
      title: siteConfig.name,
      description,
      images: [ogImageUrl],
    },
    icons: {
      icon: [
        {
          url: LIGHT_ICON_URL,
          media: "(prefers-color-scheme: light)",
          type: "image/svg+xml",
        },
        {
          url: DARK_ICON_URL,
          media: "(prefers-color-scheme: dark)",
          type: "image/svg+xml",
        },
        {
          url: PNG_ICON_URL,
          sizes: "256x256",
          type: "image/png",
        },
      ],
      shortcut: PNG_ICON_URL,
      apple: PNG_ICON_URL,
    },
    manifest: withUiBasePath("/site.webmanifest"),
  }
}

export default async function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  const locale = await getLocale()

  return (
    <html lang={locale} suppressHydrationWarning className={fontVariables}>
      <head>
        <Script
          id="theme-layout-init"
          strategy="beforeInteractive"
          dangerouslySetInnerHTML={{
            __html: `
              try {
                const isDark = localStorage.theme === 'dark' || ((!('theme' in localStorage) || localStorage.theme === 'system') && window.matchMedia('(prefers-color-scheme: dark)').matches)
                if (isDark) {
                  document.querySelector('meta[name="theme-color"]').setAttribute('content', '${META_THEME_COLORS.dark}')
                }
                const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches
                const favicon = document.querySelector('link[data-theme-favicon="true"]') || document.createElement('link')
                favicon.setAttribute('rel', 'icon')
                favicon.setAttribute('type', 'image/svg+xml')
                favicon.setAttribute('data-theme-favicon', 'true')
                favicon.setAttribute('href', systemDark ? ${JSON.stringify(DARK_ICON_URL)} : ${JSON.stringify(LIGHT_ICON_URL)})
                document.head.appendChild(favicon)
              } catch (_) {}
            `,
          }}
        />
        <meta name="theme-color" content={META_THEME_COLORS.light} />
      </head>
      <body
        className={cn(
          "group/body relative overscroll-none antialiased [--footer-height:--spacing(14)] [--header-height:--spacing(14)] xl:[--footer-height:--spacing(24)]"
        )}
      >
        <NextIntlClientProvider>
          <ThemeProvider>
            <ThemeFavicon />
            <ActiveThemeProvider>
              <NuqsAdapter>
                <TooltipProvider delayDuration={0}>
                  {children}
                  <Toaster position="top-center" />
                </TooltipProvider>
              </NuqsAdapter>
              <TailwindIndicator />
            </ActiveThemeProvider>
          </ThemeProvider>
        </NextIntlClientProvider>
        <Analytics />
        <div id="portal" className="fixed top-0 left-0 z-40" />
      </body>
    </html>
  )
}
