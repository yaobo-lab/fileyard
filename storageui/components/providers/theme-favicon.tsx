"use client"

import { useEffect } from "react"

import { withUiBasePath } from "@/lib/config/base-path"

const THEME_FAVICON_SELECTOR = 'link[data-theme-favicon="true"]'
const FAVICON_HREFS = {
  light: withUiBasePath("/icon.svg"),
  dark: withUiBasePath("/icon-dark.svg"),
}

function updateThemeFavicon(theme: "light" | "dark") {
  let favicon = document.querySelector<HTMLLinkElement>(THEME_FAVICON_SELECTOR)

  if (!favicon) {
    favicon = document.createElement("link")
    favicon.rel = "icon"
    favicon.type = "image/svg+xml"
    favicon.dataset.themeFavicon = "true"
    document.head.appendChild(favicon)
  }

  favicon.href = FAVICON_HREFS[theme]
}

export function ThemeFavicon() {
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)")

    const apply = () => updateThemeFavicon(media.matches ? "dark" : "light")

    apply()
    media.addEventListener("change", apply)

    return () => media.removeEventListener("change", apply)
  }, [])

  return null
}
