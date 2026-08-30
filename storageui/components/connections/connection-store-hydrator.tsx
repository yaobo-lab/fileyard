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

    const loadEnvConnections = () => {
      listEnvConnectionsAction()
        .then((envConnections) => {
          useConnectionStore.getState().setEnvConnections(envConnections)
        })
        .catch(() => {
          useConnectionStore.getState().setEnvConnections([])
        })
    }
    void Promise.resolve(useConnectionStore.persist.rehydrate()).then(
      loadEnvConnections,
      loadEnvConnections
    )

    void usePreferencesStore.persist.rehydrate()
    void useBucketBrowserStore.persist.rehydrate()
    void useFileMarksStore.persist.rehydrate()
  }, [])

  return children
}
