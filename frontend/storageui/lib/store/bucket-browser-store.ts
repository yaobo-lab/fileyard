"use client"

import { create } from "zustand"
import { createJSONStorage, persist } from "zustand/middleware"

import type { Connection } from "@/lib/storage/connections"
import type {
  FileSystemFilter,
  FileSystemSortState,
  FileSystemView,
} from "@/components/explorer/types"

const STORE_NAME = "filesystem.bucket-browser-store"
const STORE_VERSION = 1

export type BucketBrowserSettings = {
  view: FileSystemView
  sort: FileSystemSortState
  filters: FileSystemFilter[]
}

export const DEFAULT_BUCKET_BROWSER_SETTINGS: BucketBrowserSettings = {
  view: "icons",
  sort: { direction: "asc", key: "name" },
  filters: [],
}

type PersistedBucketBrowserState = {
  buckets: Record<string, BucketBrowserSettings>
}

type BucketBrowserStore = PersistedBucketBrowserState & {
  setView: (bucket: string, view: FileSystemView) => void
  setSort: (bucket: string, sort: FileSystemSortState) => void
  setFilters: (bucket: string, filters: FileSystemFilter[]) => void
}

export function bucketBrowserKey(connection: Connection): string {
  return JSON.stringify([
    connection.provider,
    connection.accountId ?? "",
    connection.endpoint ?? "",
    connection.region ?? "",
    connection.bucket,
  ])
}

function settingsFor(
  buckets: Record<string, BucketBrowserSettings>,
  bucket: string
): BucketBrowserSettings {
  return buckets[bucket] ?? DEFAULT_BUCKET_BROWSER_SETTINGS
}

export const useBucketBrowserStore = create<BucketBrowserStore>()(
  persist(
    (set) => ({
      buckets: {},
      setView: (bucket, view) =>
        set((state) => ({
          buckets: {
            ...state.buckets,
            [bucket]: { ...settingsFor(state.buckets, bucket), view },
          },
        })),
      setSort: (bucket, sort) =>
        set((state) => ({
          buckets: {
            ...state.buckets,
            [bucket]: { ...settingsFor(state.buckets, bucket), sort },
          },
        })),
      setFilters: (bucket, filters) =>
        set((state) => ({
          buckets: {
            ...state.buckets,
            [bucket]: { ...settingsFor(state.buckets, bucket), filters },
          },
        })),
    }),
    {
      name: STORE_NAME,
      version: STORE_VERSION,
      storage: createJSONStorage(() => window.localStorage),
      skipHydration: true,
      partialize: (state): PersistedBucketBrowserState => ({
        buckets: state.buckets,
      }),
    }
  )
)
