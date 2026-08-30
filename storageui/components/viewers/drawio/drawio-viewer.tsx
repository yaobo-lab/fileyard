"use client"

import * as React from "react"
import { useTranslations } from "next-intl"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import {
  AppIcon,
  MinusSignCircleIcon,
  PlusSignCircleIcon,
} from "@/components/foundations/icons"

const DRAWIO_VIEWER_ORIGIN = "https://viewer.diagrams.net"

const IMAGE_LIKE = /\.(svg|png|jpe?g|gif|webp)$/

const ZOOM_MIN = 0.2
const ZOOM_MAX = 5
const ZOOM_STEP = 0.2

type DrawioViewerProps = {
  src: string
  fileName: string
  className?: string
  isDark?: boolean
}

export function DrawioViewer({
  src,
  fileName,
  className,
  isDark = false,
}: DrawioViewerProps) {
  const isImage = IMAGE_LIKE.test(fileName.toLowerCase())

  return isImage ? (
    <DrawioImageViewer src={src} fileName={fileName} className={className} />
  ) : (
    <DrawioXmlViewer
      src={src}
      fileName={fileName}
      className={className}
      isDark={isDark}
    />
  )
}

// --- `.drawio.svg` / `.drawio.png`: image with scroll + zoom controls. --------

function DrawioImageViewer({
  src,
  fileName,
  className,
}: {
  src: string
  fileName: string
  className?: string
}) {
  const t = useTranslations("Viewer")
  const [zoom, setZoom] = React.useState(1)
  const [loaded, setLoaded] = React.useState(false)
  const [failed, setFailed] = React.useState(false)

  const clampZoom = (value: number) =>
    Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, value))

  return (
    <div className={cn("relative flex min-h-0 flex-col", className)}>
      <div className="flex min-h-0 flex-1 overflow-auto bg-(--drawio-canvas) p-4 [--drawio-canvas:#f7f8fa] dark:[--drawio-canvas:#1b1d22]">
        {failed ? (
          <div className="m-auto text-sm text-muted-foreground">
            {t("diagramError")}
          </div>
        ) : (
          <div className="m-auto">
            <img
              src={src}
              alt={fileName}
              onLoad={() => setLoaded(true)}
              onError={() => setFailed(true)}
              style={{ width: `${zoom * 100}%` }}
              className="h-auto max-w-none origin-center select-none"
              draggable={false}
            />
          </div>
        )}
        {!loaded && !failed ? (
          <div className="absolute inset-0 grid place-items-center">
            <Spinner />
          </div>
        ) : null}
      </div>
      <ZoomBar
        zoom={zoom}
        onZoomIn={() => setZoom((z) => clampZoom(z + ZOOM_STEP))}
        onZoomOut={() => setZoom((z) => clampZoom(z - ZOOM_STEP))}
        onReset={() => setZoom(1)}
      />
    </div>
  )
}

// --- `.drawio` / `.dio` XML: rendered by the embedded diagrams.net viewer. ----

function DrawioXmlViewer({
  src,
  fileName,
  className,
  isDark,
}: {
  src: string
  fileName: string
  className?: string
  isDark: boolean
}) {
  const t = useTranslations("Viewer")
  const iframeRef = React.useRef<HTMLIFrameElement>(null)
  const [xml, setXml] = React.useState<string | null>(null)
  const [status, setStatus] = React.useState<"loading" | "ready" | "error">(
    "loading"
  )

  // Pull the diagram source so it can be handed to the iframe locally.
  React.useEffect(() => {
    let cancelled = false
    setStatus("loading")
    setXml(null)

    fetch(src)
      .then((response) => {
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        return response.text()
      })
      .then((text) => {
        if (!cancelled) setXml(text)
      })
      .catch(() => {
        if (!cancelled) setStatus("error")
      })

    return () => {
      cancelled = true
    }
  }, [src])

  // The viewer announces itself with an `init` event; reply with the diagram.
  React.useEffect(() => {
    if (xml === null) return

    function handleMessage(event: MessageEvent) {
      if (event.origin !== DRAWIO_VIEWER_ORIGIN) return
      const frame = iframeRef.current
      if (!frame?.contentWindow) return

      let message: { event?: string } | null = null
      try {
        message =
          typeof event.data === "string" ? JSON.parse(event.data) : event.data
      } catch {
        return
      }

      if (message?.event === "init") {
        frame.contentWindow.postMessage(
          JSON.stringify({ action: "load", xml, autosave: 0 }),
          DRAWIO_VIEWER_ORIGIN
        )
      } else if (message?.event === "load") {
        setStatus("ready")
      }
    }

    window.addEventListener("message", handleMessage)
    return () => window.removeEventListener("message", handleMessage)
  }, [xml])

  const viewerSrc = React.useMemo(() => {
    const params = new URLSearchParams({
      embed: "1",
      proto: "json",
      spin: "1",
      lightbox: "1",
      nav: "1",
      dark: isDark ? "1" : "0",
    })
    return `${DRAWIO_VIEWER_ORIGIN}/?${params.toString()}`
  }, [isDark])

  if (status === "error") {
    return (
      <div
        className={cn(
          "grid min-h-0 flex-1 place-items-center text-sm text-muted-foreground",
          className
        )}
      >
        {t("diagramError")}
      </div>
    )
  }

  return (
    <div className={cn("relative min-h-0 flex-1", className)}>
      {xml !== null ? (
        <iframe
          ref={iframeRef}
          src={viewerSrc}
          title={fileName}
          className="h-full w-full border-0 bg-[#f7f8fa] dark:bg-[#1b1d22]"
          sandbox="allow-scripts allow-same-origin"
        />
      ) : null}
      {status !== "ready" ? (
        <div className="absolute inset-0 grid place-items-center bg-background">
          <Spinner />
        </div>
      ) : null}
    </div>
  )
}

function ZoomBar({
  zoom,
  onZoomIn,
  onZoomOut,
  onReset,
}: {
  zoom: number
  onZoomIn: () => void
  onZoomOut: () => void
  onReset: () => void
}) {
  const t = useTranslations("Viewer")
  return (
    <div className="flex shrink-0 items-center justify-center gap-1 border-t px-3 py-1.5">
      <Button
        size="icon-sm"
        variant="ghost"
        aria-label={t("zoomOut")}
        title={t("zoomOut")}
        onClick={onZoomOut}
        disabled={zoom <= ZOOM_MIN}
      >
        <AppIcon icon={MinusSignCircleIcon} />
      </Button>
      <button
        type="button"
        onClick={onReset}
        className="min-w-12 rounded px-1.5 py-0.5 text-xs text-muted-foreground tabular-nums transition-colors hover:bg-accent"
        title={t("resetZoom")}
      >
        {Math.round(zoom * 100)}%
      </button>
      <Button
        size="icon-sm"
        variant="ghost"
        aria-label={t("zoomIn")}
        title={t("zoomIn")}
        onClick={onZoomIn}
        disabled={zoom >= ZOOM_MAX}
      >
        <AppIcon icon={PlusSignCircleIcon} />
      </Button>
    </div>
  )
}
