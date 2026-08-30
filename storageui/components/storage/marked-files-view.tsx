"use client"

import * as React from "react"
import { useTranslations } from "next-intl"

import type { MarkedFile } from "@/lib/store/file-marks-store"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  FileSystemIconSpriteSheet,
  FileTypeIcon,
} from "@/components/explorer/file-system"
import {
  AppIcon,
  Clock01Icon,
  FavouriteIcon,
} from "@/components/foundations/icons"
import { NEUTRAL_BADGE_CLASSNAME } from "@/components/storage/badge-styles"

type MarkedFilesViewProps = {
  section: "recents" | "starred"
  connectionName: string
  headerLeading?: React.ReactNode
  files: MarkedFile[]
  isStarredAction: (key: string) => boolean
  onOpenAction: (file: MarkedFile) => void
  onToggleStarAction: (file: MarkedFile) => void
  onClearRecentsAction?: () => void
  showFileExtensions: boolean
}

function basename(path: string) {
  const trimmed = path.endsWith("/") ? path.slice(0, -1) : path
  const index = trimmed.lastIndexOf("/")
  return index === -1 ? trimmed : trimmed.slice(index + 1)
}

function displayName(file: MarkedFile, showFileExtensions: boolean) {
  const name = file.name ?? basename(file.path)
  if (showFileExtensions) return name
  const dotIndex = name.lastIndexOf(".")
  return dotIndex <= 0 ? name : name.slice(0, dotIndex)
}

function formatSize(size: number | undefined) {
  if (size === undefined) return null
  if (size < 1024) return `${size} B`
  const units = ["KB", "MB", "GB", "TB"]
  let value = size / 1024
  let unitIndex = 0
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unitIndex]}`
}

function formatDate(iso: string | undefined) {
  if (!iso) return null
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return null
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  })
}

function metaLine(file: MarkedFile) {
  return [formatSize(file.size), formatDate(file.updatedAt)]
    .filter(Boolean)
    .join(" · ")
}

export function MarkedFilesView({
  section,
  connectionName,
  headerLeading,
  files,
  isStarredAction,
  onOpenAction,
  onToggleStarAction,
  onClearRecentsAction,
  showFileExtensions,
}: MarkedFilesViewProps) {
  const t = useTranslations("Marked")
  return (
    <div className="flex h-full min-h-0 flex-col">
      <FileSystemIconSpriteSheet />
      <div className="flex h-11 shrink-0 items-center justify-between gap-2 border-b px-3">
        <div className="flex min-w-0 items-center gap-2">
          {headerLeading}
          <span className="text-sm font-medium">{t(section)}</span>
          <Badge
            variant="outline"
            className={cn(NEUTRAL_BADGE_CLASSNAME, "truncate text-sm")}
          >
            {connectionName}
          </Badge>
        </div>
        {section === "recents" && files.length > 0 ? (
          <Button size="sm" variant="ghost" onClick={onClearRecentsAction}>
            {t("clearRecents")}
          </Button>
        ) : null}
      </div>

      {files.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
          <div className="flex size-12 items-center justify-center rounded-xl bg-muted text-muted-foreground">
            <AppIcon
              icon={section === "recents" ? Clock01Icon : FavouriteIcon}
              className="size-6"
            />
          </div>
          <div className="space-y-1">
            <h2 className="text-base font-semibold">
              {section === "recents" ? t("noRecents") : t("noStarred")}
            </h2>
            <p className="max-w-sm text-sm text-muted-foreground">
              {section === "recents" ? t("recentsHint") : t("starredHint")}
            </p>
          </div>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto">
          <div className="divide-y">
            {files.map((file) => {
              const starred = isStarredAction(file.key)
              return (
                <div
                  key={file.key}
                  role="button"
                  tabIndex={0}
                  onClick={() => onOpenAction(file)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault()
                      onOpenAction(file)
                    }
                  }}
                  className="group flex cursor-pointer items-center gap-3 px-3 py-2.5 outline-none hover:bg-accent/50 focus-visible:bg-accent/50"
                >
                  <FileTypeIcon
                    fileName={file.name ?? basename(file.path)}
                    className="size-7"
                  />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">
                      {displayName(file, showFileExtensions)}
                    </p>
                    <p className="truncate text-xs text-muted-foreground">
                      {metaLine(file) || file.path}
                    </p>
                  </div>
                  <Button
                    size="icon-sm"
                    variant="ghost"
                    aria-label={starred ? t("removeStar") : t("addStar")}
                    title={starred ? t("removeStar") : t("addStar")}
                    onClick={(event) => {
                      event.stopPropagation()
                      onToggleStarAction(file)
                    }}
                    className={cn(
                      "shrink-0 transition-opacity",
                      starred
                        ? "text-amber-500 hover:text-amber-500"
                        : "text-muted-foreground opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
                    )}
                  >
                    <AppIcon
                      icon={FavouriteIcon}
                      className={starred ? "fill-current" : undefined}
                    />
                  </Button>
                </div>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
