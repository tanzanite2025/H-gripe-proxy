import dayjs from 'dayjs'
import { memo, useSyncExternalStore } from 'react'

type TickListener = () => void

let tickNow = Date.now()
const tickListeners = new Set<TickListener>()
let tickTimer: ReturnType<typeof setInterval> | null = null

const startTick = () => {
  if (tickTimer !== null) return

  tickTimer = setInterval(() => {
    tickNow = Date.now()
    tickListeners.forEach((listener) => listener())
  }, 5000)
}

const stopTick = () => {
  if (tickListeners.size === 0 && tickTimer !== null) {
    clearInterval(tickTimer)
    tickTimer = null
  }
}

const tickStore = {
  subscribe: (listener: TickListener) => {
    tickListeners.add(listener)
    startTick()

    return () => {
      tickListeners.delete(listener)
      stopTick()
    }
  },
  getSnapshot: () => tickNow,
}

interface RelativeTimeCellProps {
  start: string
}

export const RelativeTimeCell = memo(function RelativeTimeCell({
  start,
}: RelativeTimeCellProps) {
  const now = useSyncExternalStore(tickStore.subscribe, tickStore.getSnapshot)
  return <>{dayjs(start).from(now)}</>
})
