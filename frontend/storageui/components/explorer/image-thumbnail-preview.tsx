"use client"

import * as React from "react"

import {
  DEFAULT_THUMBNAIL_WIDTH,
  thumbnailUrl,
  thumbnailWidthForTile,
} from "@/lib/storage/thumbnails"
import { FileTypeIcon } from "@/components/explorer/internals"
import type { FileSystemFileItem } from "@/components/explorer/types"

type ImageThumbnailPreviewProps = {
  cacheKey: string
  file: FileSystemFileItem
  getFileUrl: (file: FileSystemFileItem) => string | Promise<string>
  urlCache: Map<string, string>
  /** `null` falls back to the presigned original. */
  thumbnailHandle?: string | null
  /** Rendered width in CSS px. */
  widthHint?: number
}

export function ImageThumbnailPreview({
  cacheKey,
  file,
  getFileUrl,
  urlCache,
  thumbnailHandle,
  widthHint,
}: ImageThumbnailPreviewProps) {
  const [thumbnailFailed, setThumbnailFailed] = React.useState(false)

  const width = widthHint
    ? thumbnailWidthForTile(widthHint)
    : DEFAULT_THUMBNAIL_WIDTH
  const objectKey = file.key ?? file.path

  const serverThumbnail =
    thumbnailHandle && !thumbnailFailed
      ? thumbnailUrl(thumbnailHandle, objectKey, width, file.etag)
      : null

  React.useEffect(() => {
    setThumbnailFailed(false)
  }, [thumbnailHandle])

  if (serverThumbnail) {
    return (
      <img
        src={serverThumbnail}
        alt=""
        draggable={false}
        loading="lazy"
        decoding="async"
        className="size-full object-cover"
        onError={() => setThumbnailFailed(true)}
      />
    )
  }

  return (
    <OriginalImagePreview
      cacheKey={cacheKey}
      file={file}
      getFileUrl={getFileUrl}
      urlCache={urlCache}
    />
  )
}

function FileGlyph({ file }: { file: FileSystemFileItem }) {
  return (
    <div className="flex size-full items-center justify-center bg-white dark:bg-neutral-100">
      <FileTypeIcon
        fileName={file.name ?? file.path}
        className="size-1/3 min-h-4 min-w-4"
      />
    </div>
  )
}

function OriginalImagePreview({
  cacheKey,
  file,
  getFileUrl,
  urlCache,
}: Pick<
  ImageThumbnailPreviewProps,
  "cacheKey" | "file" | "getFileUrl" | "urlCache"
>) {
  const knownUrl = file.url ?? urlCache.get(cacheKey) ?? null
  const [url, setUrl] = React.useState<string | null>(knownUrl)
  const [failed, setFailed] = React.useState(false)

  const fileRef = React.useRef(file)
  React.useEffect(() => {
    fileRef.current = file
  })

  const filePath = file.path
  const fileUrl = file.url ?? null

  // Keyed by path, not object identity: the manifest re-creates file objects,
  // and re-running on identity would duplicate an in-flight presign.
  React.useEffect(() => {
    const cachedUrl = fileUrl ?? urlCache.get(cacheKey) ?? null
    if (cachedUrl) {
      setUrl(cachedUrl)
      setFailed(false)
      return
    }

    let isCurrent = true
    setUrl(null)
    setFailed(false)

    void Promise.resolve(getFileUrl(fileRef.current))
      .then((nextUrl) => {
        if (!nextUrl) throw new Error("No preview URL")
        urlCache.set(cacheKey, nextUrl)
        if (isCurrent) setUrl(nextUrl)
      })
      .catch(() => {
        if (isCurrent) setFailed(true)
      })

    return () => {
      isCurrent = false
    }
  }, [cacheKey, filePath, fileUrl, getFileUrl, urlCache])

  if (url && !failed) {
    return (
      <img
        src={url}
        alt=""
        draggable={false}
        loading="lazy"
        decoding="async"
        className="size-full object-cover"
        onError={() => {
          urlCache.delete(cacheKey)
          setFailed(true)
        }}
      />
    )
  }

  if (!failed) {
    return (
      <div
        aria-hidden="true"
        className="size-full animate-pulse bg-muted motion-reduce:animate-none"
      />
    )
  }

  return <FileGlyph file={file} />
}
