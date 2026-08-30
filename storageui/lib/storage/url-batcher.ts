/**
 * Collapses the presign burst a viewport produces into one call: a grid fires one
 * `getFileUrl` per tile in the same commit, and server actions are dispatched
 * through a queue, making those sequential round trips.
 */

export type UrlBatcher = {
  /** Resolves to `""` when the key could not be signed. */
  get: (key: string) => Promise<string>
}

export type UrlBatcherOptions = {
  windowMs?: number
  /** Overflow goes to the next batch. */
  maxBatchSize?: number
}

type Deferred = {
  promise: Promise<string>
  resolve: (url: string) => void
  reject: (error: unknown) => void
}

function deferred(): Deferred {
  let resolve!: (url: string) => void
  let reject!: (error: unknown) => void
  const promise = new Promise<string>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

export function createUrlBatcher(
  signMany: (keys: string[]) => Promise<Record<string, string>>,
  { windowMs = 8, maxBatchSize = 64 }: UrlBatcherOptions = {}
): UrlBatcher {
  const queued = new Map<string, Deferred>()
  /** Every key with an unsettled promise, queued or already sent. */
  const outstanding = new Map<string, Promise<string>>()
  let timer: ReturnType<typeof setTimeout> | null = null

  function schedule() {
    if (timer === null) timer = setTimeout(flush, windowMs)
  }

  function flush() {
    timer = null
    if (queued.size === 0) return

    const batch: Array<[string, Deferred]> = []
    for (const [key, deferral] of queued) {
      if (batch.length >= maxBatchSize) break
      batch.push([key, deferral])
      queued.delete(key)
    }

    if (queued.size > 0) schedule()

    void signMany(batch.map(([key]) => key))
      .then((urls) => {
        for (const [key, deferral] of batch) {
          outstanding.delete(key)
          deferral.resolve(urls[key] ?? "")
        }
      })
      .catch((error: unknown) => {
        for (const [key, deferral] of batch) {
          outstanding.delete(key)
          deferral.reject(error)
        }
      })
  }

  return {
    get(key) {
      const existing = outstanding.get(key)
      if (existing) return existing

      const deferral = deferred()
      queued.set(key, deferral)
      outstanding.set(key, deferral.promise)
      schedule()
      return deferral.promise
    },
  }
}
