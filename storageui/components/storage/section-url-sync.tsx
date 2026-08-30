"use client"

import * as React from "react"

import { useNavStore, type BrowseSection } from "@/lib/store/nav-store"

const BASE_PATH = process.env.NEXT_PUBLIC_BASE_PATH ?? ""

const SECTION_PATH: Record<BrowseSection, string> = {
  all: BASE_PATH || "/",
  recents: `${BASE_PATH}/recents`,
  starred: `${BASE_PATH}/starred`,
}

function sectionFromPathname(pathname: string): BrowseSection {
  let rest = pathname
  if (BASE_PATH && rest.startsWith(BASE_PATH))
    rest = rest.slice(BASE_PATH.length)
  const segment = rest.replace(/^\/+|\/+$/g, "").toLowerCase()
  if (segment === "recents") return "recents"
  if (segment === "starred") return "starred"
  return "all"
}

export function SectionUrlSync() {
  const section = useNavStore((state) => state.section)
  const setSection = useNavStore((state) => state.setSection)
  const firstRun = React.useRef(true)

  React.useEffect(() => {
    setSection(sectionFromPathname(window.location.pathname))
  }, [setSection])

  React.useEffect(() => {
    if (firstRun.current) {
      firstRun.current = false
      return
    }
    const target = SECTION_PATH[section]
    if (window.location.pathname !== target) {
      window.history.pushState(null, "", target)
    }
  }, [section])

  React.useEffect(() => {
    const onPopState = () => {
      setSection(sectionFromPathname(window.location.pathname))
    }
    window.addEventListener("popstate", onPopState)
    return () => window.removeEventListener("popstate", onPopState)
  }, [setSection])

  return null
}
