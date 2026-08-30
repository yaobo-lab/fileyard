import * as React from "react"
import {
  CollisionPriority,
  CollisionType,
  type CollisionDetector,
} from "@dnd-kit/abstract"

import { cn } from "@/lib/utils"
import type {
  FileEntry,
  FileSystemEntry,
  FileSystemFileItem,
} from "@/components/explorer/types"

// Drop hit-test for folders. The default detector fires as soon as the pointer
// enters the folder's whole box (tile or row), which makes it easy to mis-drop
// by grazing the edge. This only counts a hit when the pointer is inside the
// central region, shrinking the active area on every side.
const FOLDER_DROP_INSET = 0.25

export const folderDropCollision: CollisionDetector = ({
  dragOperation,
  droppable,
}) => {
  const pointer = dragOperation.position.current
  const shape = droppable.shape
  if (!pointer || !shape) return null

  const rect = shape.boundingRectangle
  const insetX = rect.width * FOLDER_DROP_INSET
  const insetY = rect.height * FOLDER_DROP_INSET
  if (
    pointer.x < rect.left + insetX ||
    pointer.x > rect.right - insetX ||
    pointer.y < rect.top + insetY ||
    pointer.y > rect.bottom - insetY
  ) {
    return null
  }

  const distance =
    Math.hypot(pointer.x - shape.center.x, pointer.y - shape.center.y) || 1
  return {
    id: droppable.id,
    value: 1 / distance,
    type: CollisionType.PointerIntersection,
    priority: CollisionPriority.High,
  }
}

export function useResolvedFileUrl(
  file: FileEntry | null,
  getFileUrl?: (file: FileSystemFileItem) => string | Promise<string>,
  cache?: Map<string, string>
) {
  const [state, setState] = React.useState<{
    isResolving: boolean
    url: string | null
  }>(() => ({
    isResolving: false,
    url: file ? (file.url ?? cache?.get(file.path) ?? null) : null,
  }))
  const fileRef = React.useRef(file)

  React.useEffect(() => {
    fileRef.current = file
  })

  const filePath = file?.path ?? null
  const fileUrl = file?.url ?? null

  React.useEffect(() => {
    const currentFile = fileRef.current
    const knownUrl =
      fileUrl ?? (filePath ? (cache?.get(filePath) ?? null) : null)

    if (!currentFile || knownUrl || !getFileUrl) {
      setState({ isResolving: false, url: knownUrl })
      return
    }

    let isCurrent = true

    setState({ isResolving: true, url: null })
    void Promise.resolve(getFileUrl(currentFile))
      .then((url) => {
        if (url) cache?.set(currentFile.path, url)
        if (isCurrent) setState({ isResolving: false, url })
      })
      .catch(() => {
        if (isCurrent) setState({ isResolving: false, url: null })
      })

    return () => {
      isCurrent = false
    }
  }, [cache, filePath, fileUrl, getFileUrl])

  return state
}

// Returns `value` once it has stopped changing for `delay` ms. Gallery
// navigation scrubs past files quickly; heavy previews (document viewers,
// presigned URL resolution) only kick in for the file the user lands on.
export function useSettledValue<T>(value: T, delay: number): T {
  const [settled, setSettled] = React.useState(value)

  React.useEffect(() => {
    if (Object.is(settled, value)) return

    const timeout = window.setTimeout(() => setSettled(value), delay)

    return () => window.clearTimeout(timeout)
  }, [delay, settled, value])

  return settled
}

export function FileSystemEmptyState({
  label,
  isLoading = false,
}: {
  label: string
  isLoading?: boolean
}) {
  return (
    <div
      className={cn(
        "flex size-full items-center justify-center text-sm text-muted-foreground",
        isLoading && "animate-pulse motion-reduce:animate-none"
      )}
    >
      {label}
    </div>
  )
}

export const ARROW_KEYS = new Set([
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "ArrowUp",
])

// Type-ahead buffers reset after this idle period, like Finder.
export const TYPE_AHEAD_RESET_MS = 700

// Letters and digits only — the same key test the tree uses — so shortcuts
// and whitespace scrolling stay untouched.
export function isTypeAheadKey(event: React.KeyboardEvent) {
  return (
    event.key.length === 1 &&
    /^[\p{L}\p{N}]$/u.test(event.key) &&
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey
  )
}

// Shared Finder-style type-ahead used by every view: printable keys
// accumulate a buffer that jumps to the next entry whose name starts with
// it, and repeating a single letter cycles through entries with that
// prefix. Each view passes its own display-ordered candidate list, so the
// same keystrokes land on the same file everywhere.
export function useEntryTypeAhead() {
  const stateRef = React.useRef({ buffer: "", timeout: 0 })

  React.useEffect(() => {
    const state = stateRef.current

    return () => window.clearTimeout(state.timeout)
  }, [])

  return React.useCallback(
    (
      event: React.KeyboardEvent,
      entries: readonly FileSystemEntry[],
      currentIndex: number
    ) => {
      if (!isTypeAheadKey(event) || entries.length === 0) return null

      // Embedded viewers (and any future inputs) keep their keystrokes.
      const target = event.target

      if (
        target instanceof HTMLElement &&
        (target.isContentEditable ||
          target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT")
      ) {
        return null
      }

      const state = stateRef.current

      window.clearTimeout(state.timeout)
      state.timeout = window.setTimeout(() => {
        state.buffer = ""
      }, TYPE_AHEAD_RESET_MS)
      state.buffer += event.key.toLowerCase()

      // A repeated single letter advances past the current entry; a longer
      // buffer refines the match in place.
      const startIndex =
        currentIndex < 0
          ? 0
          : currentIndex + (state.buffer.length === 1 ? 1 : 0)

      for (let step = 0; step < entries.length; step += 1) {
        const entry = entries[(startIndex + step) % entries.length]

        if (entry.name.toLowerCase().startsWith(state.buffer)) {
          event.preventDefault()
          return entry
        }
      }
      event.preventDefault()
      return null
    },
    []
  )
}

// Selects (and focuses) the entry reached by an arrow key. Up/down use row
// geometry so navigation follows the rendered auto-fill grid.
export function moveGridSelection({
  entries,
  itemRefs,
  key,
  onSelect,
  selectedPath,
}: {
  entries: FileSystemEntry[]
  itemRefs: Map<string, HTMLButtonElement>
  key: string
  onSelect: (entry: FileSystemEntry | null) => void
  selectedPath: string | null
}) {
  if (entries.length === 0) return false

  const currentIndex = entries.findIndex((entry) => entry.path === selectedPath)
  let nextEntry: FileSystemEntry | undefined

  if (currentIndex === -1) {
    nextEntry = entries[0]
  } else if (key === "ArrowLeft" || key === "ArrowRight") {
    nextEntry = entries[currentIndex + (key === "ArrowLeft" ? -1 : 1)]
  } else {
    const currentElement = itemRefs.get(entries[currentIndex].path)

    if (!currentElement) return false

    const currentRect = currentElement.getBoundingClientRect()
    let bestScore = Infinity

    for (const entry of entries) {
      if (entry.path === selectedPath) continue

      const rect = itemRefs.get(entry.path)?.getBoundingClientRect()

      if (!rect) continue

      const rowDelta =
        key === "ArrowDown"
          ? rect.top - currentRect.top
          : currentRect.top - rect.top

      if (rowDelta <= 1) continue

      const score = rowDelta * 1000 + Math.abs(rect.left - currentRect.left)

      if (score < bestScore) {
        bestScore = score
        nextEntry = entry
      }
    }
  }

  if (!nextEntry) return false

  onSelect(nextEntry)
  itemRefs.get(nextEntry.path)?.focus()
  return true
}

// Icon grid geometry (px at the default 16px root font size). Tiles have a
// fixed height — a 4rem glyph box plus a reserved two-line label — so rows
// share one stride and the grid can window cleanly.
export const ICON_GRID_PADDING = 12 // p-3
export const ICON_MIN_TILE_WIDTH = 104 // 6.5rem
export const ICON_TILE_GAP_X = 4 // gap-x-1
export const ICON_TILE_HEIGHT = 102 // h-16 glyph box + gap-1.5 + two text-xs lines
export const ICON_ROW_GAP = 12 // gap-y-3
export const ICON_ROW_STRIDE = ICON_TILE_HEIGHT + ICON_ROW_GAP
