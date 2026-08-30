"use client"

// Bridges the sidebar's Upload button to the file browser's hidden file input.
// The browser owns the <input> (and the current folder); the sidebar just calls
// the registered opener synchronously within its click handler so the native
// file picker still counts as a user gesture. Not persisted.
import { create } from "zustand"

type UploadUiStore = {
  pickFiles: (() => void) | null
  setPickFiles: (fn: (() => void) | null) => void
}

export const useUploadUiStore = create<UploadUiStore>((set) => ({
  pickFiles: null,
  setPickFiles: (fn) => set({ pickFiles: fn }),
}))
