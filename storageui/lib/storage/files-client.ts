import type { Files } from "files-sdk"
import type { ZipApi } from "files-sdk/zip"

/**
 * A fully-built `files-sdk` client with the ZIP plugin. Constructed server-side
 * (see `connections-server.ts`) or, for direct client requests, in the browser
 * (see `connections-client.ts`); the interface is identical either way.
 */
export type FilesClient = Files & ZipApi

/** Minimal, serializable description of an entry to operate on. */
export type EntryRef = { kind: "file" | "folder"; path: string; key?: string }

/** A presigned direct upload, as returned by `files-sdk`'s `signedUploadUrl`. */
export type SignedUpload =
  | { method: "PUT"; url: string; headers?: Record<string, string> }
  | { method: "POST"; url: string; fields: Record<string, string> }
