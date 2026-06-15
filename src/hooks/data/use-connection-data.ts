import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect } from 'react'

import {
  subscribeMetrics,
  type ConnectionMetricsEventPayload,
} from '@/services/connection-metrics'

const MAX_CLOSED_CONNS_NUM = 500
const QUERY_KEY = 'rust-connection-metrics'

export const initConnData: ConnectionMonitorData = {
  uploadTotal: 0,
  downloadTotal: 0,
  activeConnections: [],
  closedConnections: [],
}

export interface ConnectionMonitorData {
  uploadTotal: number
  downloadTotal: number
  activeConnections: IConnectionsItem[]
  closedConnections: IConnectionsItem[]
}

// ── hook ─────────────────────────────────────────────────────────────

export const useConnectionData = () => {
  const queryClient = useQueryClient()

  useEffect(() => {
    return subscribeMetrics((payload) => {
      queryClient.setQueryData<ConnectionMonitorData>(
        [QUERY_KEY],
        (prev = initConnData) => mergeFromRustEvent(payload, prev),
      )
    })
  }, [queryClient])

  const response = useQuery<ConnectionMonitorData>({
    queryKey: [QUERY_KEY],
    queryFn: () => initConnData,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
  })

  const clearClosedConnections = useCallback(() => {
    queryClient.setQueryData<ConnectionMonitorData>([QUERY_KEY], (prev) => ({
      uploadTotal: prev?.uploadTotal ?? 0,
      downloadTotal: prev?.downloadTotal ?? 0,
      activeConnections: prev?.activeConnections ?? [],
      closedConnections: [],
    }))
  }, [queryClient])

  const refresh = useCallback(() => {
    void import('@/services/connection-metrics').then(({ resetConnectionMetrics }) =>
      resetConnectionMetrics(),
    )
  }, [])

  return {
    response,
    refreshGetClashConnection: refresh,
    clearClosedConnections,
  }
}

// ── merge logic ──────────────────────────────────────────────────────

function mergeFromRustEvent(
  payload: ConnectionMetricsEventPayload,
  previous: ConnectionMonitorData,
): ConnectionMonitorData {
  const nextConnections = payload.raw.connections ?? []
  const previousActive = previous.activeConnections ?? []
  const previousClosed = previous.closedConnections ?? []

  const speedMap = new Map<string, { curUpload: number; curDownload: number }>()
  for (const s of payload.metrics.speeds) {
    speedMap.set(s.id, { curUpload: s.curUpload, curDownload: s.curDownload })
  }

  const nextById = new Map<string, IConnectionsItem>()
  for (const conn of nextConnections) {
    nextById.set(conn.id, conn)
  }

  const prevActiveMap = new Map<string, IConnectionsItem>()
  for (const prev of previousActive) {
    prevActiveMap.set(prev.id, prev)
  }

  const dropped: IConnectionsItem[] = []
  for (const prev of previousActive) {
    if (!nextById.has(prev.id)) {
      dropped.push(prev)
    }
  }

  const activeConnections: IConnectionsItem[] = nextConnections.map((conn) => {
    const speed = speedMap.get(conn.id)
    const prev = prevActiveMap.get(conn.id)
    if (prev && prev.upload === conn.upload && prev.download === conn.download) {
      return prev
    }
    return {
      ...conn,
      curUpload: speed?.curUpload ?? 0,
      curDownload: speed?.curDownload ?? 0,
    }
  })

  const rawClosedLen = previousClosed.length + dropped.length
  let closedConnections: IConnectionsItem[]
  if (rawClosedLen <= MAX_CLOSED_CONNS_NUM) {
    closedConnections = previousClosed.concat(dropped)
  } else {
    const skipPrev = rawClosedLen - MAX_CLOSED_CONNS_NUM
    closedConnections =
      skipPrev >= previousClosed.length
        ? dropped.slice(skipPrev - previousClosed.length)
        : previousClosed.slice(skipPrev).concat(dropped)
  }

  return {
    uploadTotal: payload.raw.uploadTotal ?? 0,
    downloadTotal: payload.raw.downloadTotal ?? 0,
    activeConnections,
    closedConnections,
  }
}
