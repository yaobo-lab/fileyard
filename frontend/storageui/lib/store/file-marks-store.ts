"use client"

import { create } from "zustand"
import { createJSONStorage, persist } from "zustand/middleware"

const STORE_NAME = "filesystem.file-marks-store"
const STORE_VERSION = 1
const RECENTS_LIMIT = 50

export type MarkedFile = {
  /** Real S3/R2 object key — used to resolve URLs and to dedupe. */
  key: string
  /** Display/canonical path. */
  path: string
  name?: string
  contentType?: string
  size?: number
  updatedAt?: string
}

export type RecentFile = MarkedFile & { openedAt: number }
export type StarredFile = MarkedFile & { starredAt: number }

type BucketMarks = {
  recents: RecentFile[]
  starred: StarredFile[]
}

type PersistedState = {
  buckets: Record<string, BucketMarks>
}

type FileMarksStore = PersistedState & {
  recordRecent: (bucket: string, file: MarkedFile) => void
  toggleStar: (bucket: string, file: MarkedFile) => void
  removeRecent: (bucket: string, key: string) => void
  clearRecents: (bucket: string) => void
}

const EMPTY_MARKS: BucketMarks = { recents: [], starred: [] }

function marksFor(
  buckets: Record<string, BucketMarks>,
  bucket: string
): BucketMarks {
  return buckets[bucket] ?? EMPTY_MARKS
}

export const useFileMarksStore = create<FileMarksStore>()(
  persist(
    (set) => ({
      buckets: {},

      recordRecent: (bucket, file) =>
        set((state) => {
          const marks = marksFor(state.buckets, bucket)
          const rest = marks.recents.filter((r) => r.key !== file.key)
          const recents = [{ ...file, openedAt: Date.now() }, ...rest].slice(
            0,
            RECENTS_LIMIT
          )
          return {
            buckets: { ...state.buckets, [bucket]: { ...marks, recents } },
          }
        }),

      toggleStar: (bucket, file) =>
        set((state) => {
          const marks = marksFor(state.buckets, bucket)
          const isStarred = marks.starred.some((s) => s.key === file.key)
          const starred = isStarred
            ? marks.starred.filter((s) => s.key !== file.key)
            : [{ ...file, starredAt: Date.now() }, ...marks.starred]
          return {
            buckets: { ...state.buckets, [bucket]: { ...marks, starred } },
          }
        }),

      removeRecent: (bucket, key) =>
        set((state) => {
          const marks = marksFor(state.buckets, bucket)
          return {
            buckets: {
              ...state.buckets,
              [bucket]: {
                ...marks,
                recents: marks.recents.filter((r) => r.key !== key),
              },
            },
          }
        }),

      clearRecents: (bucket) =>
        set((state) => {
          const marks = marksFor(state.buckets, bucket)
          return {
            buckets: { ...state.buckets, [bucket]: { ...marks, recents: [] } },
          }
        }),
    }),
    {
      name: STORE_NAME,
      version: STORE_VERSION,
      storage: createJSONStorage(() => window.localStorage),
      skipHydration: true,
      partialize: (state): PersistedState => ({ buckets: state.buckets }),
    }
  )
)
