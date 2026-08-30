import type { ConnectionRef } from "@/lib/storage/connection-ref"
import { resolveFiles } from "@/lib/storage/connections-server"
import { collectZipKeys } from "@/lib/storage/file-operations"

export const dynamic = "force-dynamic"

type ZipRequest = {
  ref: ConnectionRef
  /** Folder prefix to bundle. Everything under it (minus markers) is zipped. */
  path: string
}

export async function POST(request: Request) {
  let body: ZipRequest
  try {
    body = (await request.json()) as ZipRequest
  } catch {
    return new Response("Invalid request.", { status: 400 })
  }

  if (!body?.ref || typeof body.path !== "string") {
    return new Response("Invalid request.", { status: 400 })
  }

  let files
  try {
    files = resolveFiles(body.ref)
  } catch {
    return new Response("Unknown connection.", { status: 404 })
  }

  const prefix = body.path.endsWith("/") ? body.path : `${body.path}/`

  let keys: string[]
  try {
    keys = await collectZipKeys(files, prefix)
  } catch (error) {
    return new Response(
      error instanceof Error ? error.message : "Could not list folder.",
      { status: 502 }
    )
  }

  if (keys.length === 0) {
    return new Response("This folder has no files to download.", {
      status: 404,
    })
  }

  const folderName = prefix.slice(0, -1).split("/").pop() || "folder"
  const stream = files.zip(keys, { name: (key) => key.slice(prefix.length) })

  return new Response(stream, {
    headers: {
      "Content-Type": "application/zip",
      "Content-Disposition": `attachment; filename="${folderName}.zip"`,
    },
  })
}
