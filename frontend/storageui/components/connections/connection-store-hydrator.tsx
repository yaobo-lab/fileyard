"use client"

import * as React from "react"

import { useBucketBrowserStore } from "@/lib/store/bucket-browser-store"
import {
  migrateLegacyConnectionStorage,
  useConnectionStore,
} from "@/lib/store/connection-store"
import { useFileMarksStore } from "@/lib/store/file-marks-store"
import { usePreferencesStore } from "@/lib/store/preferences-store"
import { listEnvConnectionsAction } from "@/app/actions/files"

export function ConnectionStoreHydrator({
  children,
}: {
  children: React.ReactNode
}) {
  React.useEffect(() => {
    migrateLegacyConnectionStorage()

    let settled = false
    const completeHydration = (envConnections: any[] = []) => {
      if (settled) return
      settled = true
      useConnectionStore.getState().setEnvConnections(envConnections)
    }

    // 防御性保底超时：如果网络代理异常或 Server Action 挂起，1.5 秒后自动完成 hydration，避免侧边栏永久死锁在骨架屏
    const fallbackTimer = setTimeout(() => {
      completeHydration([])
    }, 1500)

    const loadEnvConnections = () => {
      listEnvConnectionsAction()
        .then((envConnections) => {
          clearTimeout(fallbackTimer)
          completeHydration(envConnections || [])
        })
        .catch((err) => {
          console.warn("[Storage] Failed to load env connections, falling back to local:", err)
          clearTimeout(fallbackTimer)
          completeHydration([])
        })
    }

    void Promise.resolve(useConnectionStore.persist.rehydrate())
      .then(loadEnvConnections)
      .catch((err) => {
        console.warn("[Storage] Failed to rehydrate connection store:", err)
        clearTimeout(fallbackTimer)
        completeHydration([])
      })

    void usePreferencesStore.persist.rehydrate()
    void useBucketBrowserStore.persist.rehydrate()
    void useFileMarksStore.persist.rehydrate()

    return () => clearTimeout(fallbackTimer)
  }, [])

  return children
}
