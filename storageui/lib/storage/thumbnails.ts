import type { Connection } from "@/lib/storage/connections"

/**
 * Sizes are a *minimum covered dimension*, not a maximum: the pipeline fits
 * `outside` the box, so the shorter side is at least this — what a slot
 * displaying with `object-cover` needs.
 */
export const THUMBNAIL_WIDTHS = [128, 256, 512, 768] as const

export type ThumbnailWidth = (typeof THUMBNAIL_WIDTHS)[number]

export const DEFAULT_THUMBNAIL_WIDTH: ThumbnailWidth = 256

export const ENV_HANDLE_PREFIX = "e_"

const BASE_PATH = process.env.NEXT_PUBLIC_BASE_PATH ?? ""

// 2 for a retina display, over 0.78 because previews render at that aspect and
// are therefore taller than they are wide.
const DEVICE_PIXELS_PER_CSS_PIXEL = 2 / 0.78

export function isThumbnailWidth(value: number): value is ThumbnailWidth {
  return (THUMBNAIL_WIDTHS as readonly number[]).includes(value)
}

/**
 * Only `env` connections can be served: their handle is just the id, derivable
 * with no round trip. A `local` connection's credentials live in the browser and
 * must not travel in a query string, so it uses presigned originals instead.
 */
export function thumbnailHandleFor(
  connection: Connection | null | undefined
): string | null {
  if (!connection || connection.source !== "env") return null
  return `${ENV_HANDLE_PREFIX}${connection.id}`
}

export function thumbnailWidthForTile(cssPixels: number): ThumbnailWidth {
  const needed = cssPixels * DEVICE_PIXELS_PER_CSS_PIXEL
  return (
    THUMBNAIL_WIDTHS.find((width) => width >= needed) ??
    THUMBNAIL_WIDTHS[THUMBNAIL_WIDTHS.length - 1]
  )
}

/** Passing `version` (the object etag) is what makes the URL cacheable forever. */
export function thumbnailUrl(
  handle: string,
  key: string,
  width: ThumbnailWidth,
  version?: string | null
): string {
  const params = new URLSearchParams({ c: handle, k: key, w: String(width) })
  if (version) params.set("v", version)
  return `${BASE_PATH}/api/thumbnail?${params.toString()}`
}
