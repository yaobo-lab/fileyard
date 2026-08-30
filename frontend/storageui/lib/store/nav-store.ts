"use client"

// Which "Browse" section the main content area shows. Shared between the
// sidebar (which sets it) and the file browser (which reads it). Ephemeral —
// it resets to "all" on reload, so it isn't persisted.
import { create } from "zustand"

export type BrowseSection = "all" | "recents" | "starred"

type NavStore = {
  section: BrowseSection
  setSection: (section: BrowseSection) => void
}

export const useNavStore = create<NavStore>((set) => ({
  section: "all",
  setSection: (section) => set({ section }),
}))
