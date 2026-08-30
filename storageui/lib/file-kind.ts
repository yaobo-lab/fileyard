import type { FileSystemFileItem } from "@/components/explorer/types"

export type FileKind =
  | "text"
  | "pdf"
  | "docx"
  | "xlsx"
  | "drawio"
  | "image"
  | "video"
  | "audio"
  | "other"

// draw.io / diagrams.net files: native `.drawio`/`.dio` XML, plus the editable
// `.drawio.svg` / `.drawio.png` exports that embed the diagram.
const DRAWIO_EXT = /\.(drawio|dio)(\.(svg|png|xml|html))?$/

const IMAGE_EXT = /\.(avif|gif|jpe?g|png|svg|webp|bmp|ico)$/
const VIDEO_EXT = /\.(mp4|webm|ogv|mov|m4v|mkv|avi)$/
const AUDIO_EXT = /\.(aac|aif|aiff|flac|m4a|mp3|oga|ogg|opus|wav|weba)$/

// Extensions we render as plain text/code in CodeMirror.
const TEXT_EXTENSIONS = new Set([
  "txt",
  "text",
  "md",
  "mdx",
  "markdown",
  "rst",
  "log",
  "csv",
  "tsv",
  "tab",
  "json",
  "jsonc",
  "json5",
  "ndjson",
  "geojson",
  "js",
  "jsx",
  "mjs",
  "cjs",
  "ts",
  "tsx",
  "mts",
  "cts",
  "html",
  "htm",
  "xml",
  "xhtml",
  "vue",
  "svelte",
  "astro",
  "css",
  "scss",
  "sass",
  "less",
  "styl",
  "yaml",
  "yml",
  "toml",
  "ini",
  "conf",
  "cfg",
  "properties",
  "env",
  "sh",
  "bash",
  "zsh",
  "fish",
  "ps1",
  "bat",
  "cmd",
  "py",
  "rb",
  "go",
  "rs",
  "java",
  "kt",
  "kts",
  "scala",
  "swift",
  "c",
  "h",
  "cc",
  "cpp",
  "hpp",
  "cxx",
  "cs",
  "m",
  "mm",
  "php",
  "pl",
  "pm",
  "lua",
  "r",
  "dart",
  "ex",
  "exs",
  "erl",
  "clj",
  "hs",
  "sql",
  "graphql",
  "gql",
  "proto",
  "prisma",
  "dockerfile",
  "makefile",
  "cmake",
  "gradle",
  "nginx",
  "lock",
  "gitignore",
  "gitattributes",
  "editorconfig",
  "npmrc",
  "diff",
  "patch",
])

// Extension-less filenames we still treat as text.
const TEXT_FILENAMES = new Set([
  "dockerfile",
  "makefile",
  "license",
  "readme",
  "changelog",
  "authors",
  "copying",
  "notice",
  "procfile",
  "gemfile",
  "rakefile",
  "brewfile",
])

const TEXT_CONTENT_TYPES = new Set([
  "application/json",
  "application/ld+json",
  "application/xml",
  "application/javascript",
  "application/x-javascript",
  "application/typescript",
  "application/x-yaml",
  "application/yaml",
  "application/toml",
  "application/x-sh",
  "application/sql",
  "application/graphql",
  "image/svg+xml", // SVG is text — show source rather than render
])

function fileName(file: FileSystemFileItem): string {
  return (file.name ?? file.path).toLowerCase()
}

function extension(name: string): string {
  const dot = name.lastIndexOf(".")
  return dot >= 0 ? name.slice(dot + 1) : ""
}

function isTextFile(file: FileSystemFileItem): boolean {
  const contentType = file.contentType?.toLowerCase() ?? ""
  if (contentType.startsWith("text/")) return true
  if (TEXT_CONTENT_TYPES.has(contentType)) return true

  const name = fileName(file)
  const base = name.slice(name.lastIndexOf("/") + 1)
  if (TEXT_FILENAMES.has(base)) return true
  return TEXT_EXTENSIONS.has(extension(name))
}

/**
 * Classify a file for the viewer dialog. The pdf/docx/xlsx/image rules mirror
 * `viewerKindForFile` in components/ui/file-system.tsx so the two stay in sync.
 */
export function getFileKind(file: FileSystemFileItem): FileKind {
  const contentType = file.contentType?.toLowerCase() ?? ""
  const name = fileName(file)

  // draw.io diagrams win over the generic image/text rules so a `.drawio.svg`
  // renders as a diagram instead of falling through to the SVG-as-text branch.
  if (DRAWIO_EXT.test(name)) return "drawio"

  // SVG is matched as text below; everything else image/* is an image.
  if (contentType.startsWith("image/") && contentType !== "image/svg+xml") {
    return "image"
  }
  if (contentType.startsWith("video/")) return "video"
  if (contentType.startsWith("audio/")) return "audio"
  if (contentType === "application/pdf" || name.endsWith(".pdf")) return "pdf"
  if (name.endsWith(".docx")) return "docx"
  if (name.endsWith(".xlsx")) return "xlsx"

  if (isTextFile(file)) return "text"
  if (IMAGE_EXT.test(name)) return "image"
  if (VIDEO_EXT.test(name)) return "video"
  if (AUDIO_EXT.test(name)) return "audio"

  return "other"
}
