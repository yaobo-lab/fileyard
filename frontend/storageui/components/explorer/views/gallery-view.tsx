import * as React from "react"
import { useTranslations } from "next-intl"

import { cn } from "@/lib/utils"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
import {
  fileKindLabel,
  FileSystemFolderGlyph,
  FileSystemViewerLoading,
  FileVisual,
  formatByteSize,
  LazyDocxViewerPreview,
  LazyPDFViewer,
  LazyXlsxViewerPreview,
  RenameContext,
  scrollIndexIntoView,
  useFormatEntryName,
  useVirtualWindow,
  viewerKindForFile,
} from "@/components/explorer/internals"
import type {
  FileEntry,
  FileSystemFileItem,
  FileSystemViewProps,
} from "@/components/explorer/types"
import { FileSystemInformation } from "@/components/explorer/views/columns-view"
import {
  useEntryTypeAhead,
  useResolvedFileUrl,
  useSettledValue,
} from "@/components/explorer/views/shared"

export const GALLERY_STRIP_PADDING = 8 // p-2
export const GALLERY_TILE_SIZE = 56 // size-14
export const GALLERY_TILE_GAP = 6 // gap-1.5
export const GALLERY_TILE_STRIDE = GALLERY_TILE_SIZE + GALLERY_TILE_GAP
// How many visited stages stay mounted so stepping back to a recent file
// restores its already-loaded preview without refetching or re-parsing;
// also bounds the memory the keep-alive pool can hold onto.
export const GALLERY_STAGE_POOL_SIZE = 4
// Of those, how many stay attached to the DOM (the active stage plus the
// two before it, keeping the usual two-or-three-file rotation instant).
// The rest wait detached, costing no layout or style-recalc work until
// they return — their page canvases remount on the way back, so returning
// to a detached stage briefly rebuilds the page content.
export const GALLERY_STAGE_ATTACHED_COUNT = 3

// The preview for one pooled file. Each stage owns its URL resolution and
// viewer state, so a mounted stage is self-contained: the root keeps
// recently shown stages alive (reparented between hosts rather than
// remounted, because the document viewers load in effects and would
// refetch and re-parse on remount) and revisiting one — in the gallery or
// the dialog — skips the presign, download, and parse work instead of
// re-running it behind a spinner. The two variants share one element
// structure, differing only in props and classes, so flipping a mounted
// stage between them keeps the viewer instance.
export function FileSystemGalleryStage({
  file,
  getFileUrl,
  loadPreviewImageUrlAction,
  pageUrlCache,
  renderFilePreview,
  toolbarActions,
  urlCache,
  variant = "stage",
}: {
  file: FileEntry
  getFileUrl?: (file: FileSystemFileItem) => string | Promise<string>
  loadPreviewImageUrlAction?: (
    file: FileSystemFileItem,
    pageIndex: number
  ) => Promise<string | null>
  pageUrlCache?: Map<string, string>
  renderFilePreview?: FileSystemViewProps["renderFilePreview"]
  /** Rendered in the viewer toolbar in the `"dialog"` variant. */
  toolbarActions?: React.ReactNode
  urlCache: Map<string, string>
  /** `"stage"` is toolbar-less in a bordered tile; `"dialog"` shows the full viewer chrome. */
  variant?: "dialog" | "stage"
}) {
  const viewerKind = viewerKindForFile(file)
  // Only viewer-backed stages need a URL; thumbnail stages render from the
  // manifest's preview images, so selecting them never triggers a presign.
  const { isResolving, url } = useResolvedFileUrl(
    viewerKind ? file : null,
    getFileUrl,
    urlCache
  )
  const isDialog = variant === "dialog"
  const [isDark, setIsDark] = React.useState(false)
  const viewerFrameClassName = cn(
    "size-full",
    !isDialog && "overflow-hidden rounded-lg border"
  )

  if (viewerKind && isResolving) {
    return <Spinner className="size-6 text-muted-foreground" />
  }
  if (viewerKind === "image" && url) {
    return (
      <img
        src={url}
        alt={file.name}
        className="max-h-full max-w-full rounded-lg object-contain"
      />
    )
  }
  if (viewerKind === "pdf" && url) {
    return (
      <div className={viewerFrameClassName}>
        <React.Suspense fallback={<FileSystemViewerLoading />}>
          <LazyPDFViewer
            src={url}
            className={cn(
              "h-full",
              isDialog && "min-h-0 overflow-hidden rounded-2xl"
            )}
            fileName={file.name}
            showToolbar={isDialog}
            showThumbnailSidebar={isDialog}
            showUpload={false}
            toolbarActions={toolbarActions}
          />
        </React.Suspense>
      </div>
    )
  }
  if (viewerKind === "docx" && url) {
    return (
      <div className={viewerFrameClassName}>
        <React.Suspense fallback={<FileSystemViewerLoading />}>
          <LazyDocxViewerPreview
            src={url}
            fileName={file.name}
            isDark={isDark}
            className={cn(
              "h-full min-h-0",
              isDialog && "overflow-hidden rounded-2xl"
            )}
            onIsDarkChangeAction={setIsDark}
            showToolbar={isDialog}
            showThumbnailSidebar={isDialog}
            showUpload={false}
            toolbarActions={toolbarActions}
          />
        </React.Suspense>
      </div>
    )
  }
  if (viewerKind === "xlsx" && url) {
    return (
      <div className={viewerFrameClassName}>
        <React.Suspense fallback={<FileSystemViewerLoading />}>
          <LazyXlsxViewerPreview
            src={url}
            fileName={file.name}
            isDark={isDark}
            className={cn(
              "h-full min-h-0",
              isDialog && "overflow-hidden rounded-2xl"
            )}
            onIsDarkChangeAction={setIsDark}
            showToolbar={isDialog}
            showUpload={false}
            toolbarActions={toolbarActions}
          />
        </React.Suspense>
      </div>
    )
  }
  return (
    <FileVisual
      file={file}
      className="w-56 max-w-full"
      loadPreviewImageUrlAction={loadPreviewImageUrlAction}
      pageable
      pageUrlCache={pageUrlCache}
      previewAspectRatio={0.78}
      renderFilePreview={renderFilePreview}
    />
  )
}

export function FileSystemGalleryView(props: FileSystemViewProps) {
  const {
    attachedStagePaths,
    entries,
    index,
    onOpen,
    onSelect,
    poolStagePath,
    registerStageHost,
    renderFilePreview,
    selectedEntry,
    selectedPath,
    selectedPaths,
  } = props
  const t = useTranslations("Explorer")
  const stripRefs = React.useRef(new Map<string, HTMLButtonElement>())
  const stripViewportRef = React.useRef<HTMLDivElement | null>(null)
  const typeAhead = useEntryTypeAhead()
  const formatName = useFormatEntryName()
  const rename = React.useContext(RenameContext)
  const activeEntry =
    selectedEntry && entries.some((entry) => entry.path === selectedEntry.path)
      ? selectedEntry
      : (entries[0] ?? null)
  const activeFile = activeEntry?.kind === "file" ? activeEntry : null
  // While arrow keys are scrubbing the strip, the center pane shows a
  // spinner; a file is only admitted to the preview pool (mounting its
  // viewer and resolving its URL) once the selection settles so each
  // keystroke stays cheap.
  const settledPath = useSettledValue(activeEntry?.path ?? null, 200)

  React.useEffect(() => {
    if (settledPath) poolStagePath(settledPath)
  }, [poolStagePath, settledPath])

  // Hosts for the root-owned preview pool: one positioned wrapper per
  // pooled path; the root reparents each live preview into its wrapper.
  // Stable callbacks per path keep React from re-running the host refs on
  // unrelated renders.
  const stageHostRefs = React.useMemo(
    () =>
      new Map(
        attachedStagePaths.map(
          (path) =>
            [
              path,
              (element: HTMLElement | null) => registerStageHost(path, element),
            ] as const
        )
      ),
    [attachedStagePaths, registerStageHost]
  )

  const activeFileSize = activeFile ? formatByteSize(activeFile.size) : null

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (entries.length === 0) return

    const currentIndex = activeEntry
      ? entries.findIndex((entry) => entry.path === activeEntry.path)
      : -1

    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
      const match = typeAhead(event, entries, currentIndex)

      if (match) {
        onSelect(match)
        // The matched tile may be outside the strip's virtual window; the
        // active-tile effect scrolls it in, and focus follows once mounted.
        requestAnimationFrame(() => stripRefs.current.get(match.path)?.focus())
      }
      return
    }

    const nextEntry =
      entries[
        currentIndex === -1
          ? 0
          : currentIndex + (event.key === "ArrowLeft" ? -1 : 1)
      ]

    if (!nextEntry) return

    onSelect(nextEntry)
    stripRefs.current.get(nextEntry.path)?.focus()
    event.preventDefault()
  }

  const { end: stripEnd, start: stripStart } = useVirtualWindow({
    count: entries.length,
    horizontal: true,
    itemStride: GALLERY_TILE_STRIDE,
    leadingPx: GALLERY_STRIP_PADDING,
    overscan: 8,
    viewportRef: stripViewportRef,
  })

  // Keep the active tile mounted and visible while scrubbing or when the
  // selection arrives from another view.
  const activePath = activeEntry?.path ?? null

  React.useLayoutEffect(() => {
    if (!activePath) return

    scrollIndexIntoView({
      horizontal: true,
      index: entries.findIndex((entry) => entry.path === activePath),
      itemSize: GALLERY_TILE_SIZE,
      itemStride: GALLERY_TILE_STRIDE,
      leadingPx: GALLERY_STRIP_PADDING,
      viewport: stripViewportRef.current,
    })
  }, [activePath, entries])

  return (
    <div className="flex size-full flex-col" onKeyDown={handleKeyDown}>
      {/* The strip comes first in DOM order (rendered below via order-last)
          so the filmstrip is the view's single tab stop: Shift+Tab exits to
          the toolbar instead of landing inside the embedded viewers. */}
      <ScrollArea
        orientation="horizontal"
        className="order-last h-auto w-full shrink-0 border-t"
        viewportRef={stripViewportRef}
        viewportClassName="p-2"
      >
        <div
          className="relative h-14 min-w-full"
          style={{
            width: entries.length
              ? entries.length * GALLERY_TILE_STRIDE - GALLERY_TILE_GAP
              : undefined,
          }}
        >
          <div
            role="listbox"
            aria-label={t("files")}
            className="absolute inset-y-0 flex items-center gap-1.5"
            style={{ left: stripStart * GALLERY_TILE_STRIDE }}
          >
            {entries.slice(stripStart, stripEnd).map((entry) => {
              // The anchor (`isActive`) drives the preview pane and tab stop;
              // every selected tile is highlighted.
              const isActive =
                entry.path === (activeEntry?.path ?? selectedPath)
              const isSelected = selectedPaths.has(entry.path) || isActive

              return (
                <button
                  key={entry.path}
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  data-file-system-path={entry.path}
                  tabIndex={isActive ? 0 : -1}
                  ref={(element) => {
                    if (element) {
                      stripRefs.current.set(entry.path, element)
                    } else {
                      stripRefs.current.delete(entry.path)
                    }
                  }}
                  onClick={(event) =>
                    onSelect(entry, {
                      toggle: event.metaKey || event.ctrlKey,
                      range: event.shiftKey,
                    })
                  }
                  onDoubleClick={() => onOpen(entry)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") onOpen(entry)
                  }}
                  title={entry.name}
                  className={cn(
                    "flex size-14 shrink-0 items-center justify-center rounded-md border border-transparent p-1 outline-none",
                    isSelected && "bg-accent",
                    isActive && "ring-2 ring-primary ring-inset"
                  )}
                >
                  {entry.kind === "folder" ? (
                    <FileSystemFolderGlyph className="h-9 w-auto" />
                  ) : (
                    <FileVisual
                      file={entry}
                      className="w-9 rounded-sm"
                      previewAspectRatio={0.78}
                      previewWidthHint={36}
                      renderFilePreview={renderFilePreview}
                    />
                  )}
                </button>
              )
            })}
          </div>
        </div>
      </ScrollArea>
      <div className="flex min-h-0 flex-1">
        <div className="relative flex min-h-0 min-w-0 flex-1 items-center justify-center p-3">
          {activeEntry?.kind === "folder" ? (
            <FileSystemFolderGlyph className="h-40 max-h-full w-auto drop-shadow-md" />
          ) : activeFile && !attachedStagePaths.includes(activeFile.path) ? (
            <Spinner className="size-6 text-muted-foreground" />
          ) : null}
          {/* Inactive hosts hide via `visibility` + `opacity`, never
              `display`: the document viewers size pages off ResizeObserver
              measurements, and display:none would collapse them to zero
              width — every reveal would re-lay-out and re-rasterize behind
              a blank pane. Stacking absolutely keeps each hidden stage at
              its real size so revealing one is pure paint. `opacity-0`
              matters: descendants can override an inherited
              visibility:hidden with their own visibility:visible (the
              spreadsheet grid's cell-selection overlay does), but nothing
              can opt out of an ancestor's zero opacity. `inert` keeps the
              hidden viewer's focusables out of reach. */}
          {attachedStagePaths.map((path) => {
            const isActiveStage = path === activeFile?.path

            return (
              <div
                key={path}
                ref={stageHostRefs.get(path)}
                inert={!isActiveStage || undefined}
                className={cn(
                  "absolute inset-0 flex items-center justify-center p-3",
                  !isActiveStage && "invisible opacity-0"
                )}
              />
            )
          })}
        </div>
        {activeEntry ? (
          <ScrollArea
            orientation="vertical"
            className="hidden w-64 shrink-0 border-l sm:block"
            viewportClassName="flex flex-col gap-3 p-4"
          >
            <div className="flex items-center gap-3">
              {activeFile ? (
                <FileVisual
                  file={activeFile}
                  className={cn(
                    "shrink-0 rounded-sm",
                    (activeFile.previewAspectRatio ?? 0.78) > 1.2
                      ? "w-16"
                      : "w-9"
                  )}
                  previewAspectRatio={0.78}
                  previewWidthHint={
                    (activeFile.previewAspectRatio ?? 0.78) > 1.2 ? 64 : 36
                  }
                  renderFilePreview={renderFilePreview}
                />
              ) : (
                <FileSystemFolderGlyph className="h-8 w-auto shrink-0" />
              )}
              <div className="min-w-0 flex-1">
                <div className="flex items-start gap-1.5 text-sm font-semibold">
                  <div
                    className="line-clamp-2 min-w-0 flex-1 wrap-break-word"
                    title={formatName(activeEntry)}
                  >
                    {formatName(activeEntry)}
                  </div>
                  {rename?.pendingPaths.has(activeEntry.path) ? (
                    <Spinner className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
                  ) : null}
                </div>
                <div className="text-xs text-muted-foreground">
                  {activeFile ? fileKindLabel(activeFile) : "Folder"}
                  {activeFileSize ? ` - ${activeFileSize}` : null}
                </div>
              </div>
            </div>
            <FileSystemInformation entry={activeEntry} index={index} />
          </ScrollArea>
        ) : null}
      </div>
    </div>
  )
}
