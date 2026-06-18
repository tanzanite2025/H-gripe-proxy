import { useQuery, useQueryClient } from '@tanstack/react-query'
import dayjs from 'dayjs'
import { useCallback, useEffect, useRef, useState } from 'react'

import { useClashLog } from '@/hooks/system'
import { getClashLogs } from '@/services/cmds'
import { subscribeCoreLogs } from '@/services/log-monitor'
import type { LogLevel } from '@/types/mihomo'
import { normalizeCoreLogLevel } from '@/utils/log-level'

import { useClash } from './use-clash'


const MAX_LOG_NUM = 1000
const FLUSH_DELAY_MS = 50
const LOG_QUERY_KEY = 'rust-core-logs'
type LogType = ILogItem['type']

const DEFAULT_LOG_TYPES: LogType[] = ['debug', 'info', 'warning', 'error']
const LOG_LEVEL_FILTERS: Record<LogLevel, LogType[]> = {
  debug: DEFAULT_LOG_TYPES,
  info: ['info', 'warning', 'error'],
  warning: ['warning', 'error'],
  error: ['error'],
  silent: [],
}

const clampLogs = (logs: ILogItem[]): ILogItem[] =>
  logs.length > MAX_LOG_NUM ? logs.slice(-MAX_LOG_NUM) : logs

const filterLogsByLevel = (
  logs: ILogItem[],
  allowedTypes: LogType[],
): ILogItem[] => {
  if (allowedTypes.length === 0) return []
  if (allowedTypes.length === DEFAULT_LOG_TYPES.length) return logs
  return logs.filter((log) => allowedTypes.includes(log.type))
}

const appendLogs = (
  current: ILogItem[] | undefined,
  incoming: ILogItem[],
): ILogItem[] => {
  const base = current ?? []
  const total = base.length + incoming.length
  if (total <= MAX_LOG_NUM) return base.concat(incoming)
  const dropFromBase = total - MAX_LOG_NUM
  if (dropFromBase >= base.length) {
    return incoming.slice(incoming.length - MAX_LOG_NUM)
  }
  return base.slice(dropFromBase).concat(incoming)
}

const normalizeLogItem = (payload: unknown): ILogItem | null => {
  if (!payload || typeof payload !== 'object') return null
  const candidate = payload as Partial<ILogItem>
  if (typeof candidate.type !== 'string' || typeof candidate.payload !== 'string') {
    return null
  }
  return {
    type: candidate.type,
    payload: candidate.payload,
    time: candidate.time,
  }
}

export const useLogData = () => {
  const queryClient = useQueryClient()
  const { clash } = useClash()
  const [clashLog] = useClashLog()
  const enableLog = clashLog.enable
  const logLevel = clash?.['log-level']
    ? normalizeCoreLogLevel(clash['log-level'])
    : clashLog.logLevel
  const allowedTypes = LOG_LEVEL_FILTERS[logLevel] ?? DEFAULT_LOG_TYPES
  const hasLoadedInitialLogsRef = useRef(false)
  const [refreshVersion, setRefreshVersion] = useState(0)
  const subscriptionCacheKey = enableLog ? LOG_QUERY_KEY : null

  const response = useQuery<ILogItem[]>({
    queryKey: [LOG_QUERY_KEY],
    queryFn: () => Promise.resolve([]),
    initialData: [],
    enabled: false,
  })

  const setLogs = useCallback(
    (updater: ILogItem[] | ((current?: ILogItem[]) => ILogItem[])) => {
      queryClient.setQueryData<ILogItem[]>([LOG_QUERY_KEY], updater)
    },
    [queryClient],
  )

  useEffect(() => {
    if (!enableLog || allowedTypes.length === 0) {
      setLogs([])
      return
    }

    let mounted = true
    let flushTimer: ReturnType<typeof setTimeout> | null = null
    const buffer: ILogItem[] = []
    let flushTimeStr: string | null = null

    const clearFlushTimer = () => {
      if (flushTimer) {
        clearTimeout(flushTimer)
        flushTimer = null
      }
    }

    const flush = () => {
      if (!buffer.length || !mounted) {
        flushTimer = null
        return
      }
      const pendingLogs = buffer.splice(0, buffer.length)
      flushTimeStr = null
      setLogs((current) => appendLogs(current, pendingLogs))
      flushTimer = null
    }

    const loadInitialLogs = async () => {
      if (hasLoadedInitialLogsRef.current) return
      const logs = await getClashLogs()
      hasLoadedInitialLogsRef.current = true
      if (!mounted) return
      setLogs((current) => {
        if (!current || current.length === 0) {
          return clampLogs(filterLogsByLevel(logs, allowedTypes))
        }
        return current
      })
    }

    void loadInitialLogs()

    const unsubscribe = subscribeCoreLogs(logLevel, (payload) => {
      const parsed = normalizeLogItem(payload)
      if (!parsed || !allowedTypes.includes(parsed.type)) {
        return
      }
      if (flushTimeStr === null) {
        flushTimeStr = dayjs().format('MM-DD HH:mm:ss')
      }
      parsed.time = flushTimeStr
      buffer.push(parsed)
      if (buffer.length > MAX_LOG_NUM) {
        buffer.splice(0, buffer.length - MAX_LOG_NUM)
      }
      if (!flushTimer) {
        flushTimer = setTimeout(flush, FLUSH_DELAY_MS)
      }
    })

    return () => {
      mounted = false
      clearFlushTimer()
      unsubscribe()
    }
  }, [allowedTypes, enableLog, logLevel, refreshVersion, setLogs])

  const refreshGetClashLog = (clear = false) => {
    if (clear) {
      if (subscriptionCacheKey) {
        queryClient.setQueryData<ILogItem[]>([subscriptionCacheKey], [])
      }
    } else {
      hasLoadedInitialLogsRef.current = false
      setRefreshVersion((version) => version + 1)
    }
  }

  return { response, refreshGetClashLog }
}
