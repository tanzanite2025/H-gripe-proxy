import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { useLocation } from 'react-router'

import { throttle } from '../utils/helpers'

interface UseScrollPositionOptions {
  mode: string
  isChainMode: boolean
  activeSelectedGroup: string | null
  renderListLength: number
}

const SCROLL_POSITION_STORAGE_KEY = 'proxy-scroll-positions'
const SCROLL_TOP_VISIBILITY_THRESHOLD = 100

const buildScrollPositionKey = (
  mode: string,
  isChainMode: boolean,
  activeSelectedGroup: string | null,
) =>
  isChainMode
    ? `${mode}:chain:${activeSelectedGroup ?? 'all'}`
    : `${mode}:normal`

const loadStoredScrollPositions = () => {
  try {
    const savedPositions = localStorage.getItem(SCROLL_POSITION_STORAGE_KEY)
    return savedPositions
      ? (JSON.parse(savedPositions) as Record<string, number>)
      : {}
  } catch (error) {
    console.error('Error restoring scroll position:', error)
    return {}
  }
}

const saveStoredScrollPositions = (positions: Record<string, number>) => {
  try {
    localStorage.setItem(
      SCROLL_POSITION_STORAGE_KEY,
      JSON.stringify(positions),
    )
  } catch (error) {
    console.error('Error saving scroll position:', error)
  }
}

export function useScrollPosition(options: UseScrollPositionOptions) {
  const { mode, isChainMode, activeSelectedGroup, renderListLength } = options
  const { pathname } = useLocation()

  const parentRef = useRef<HTMLDivElement>(null)
  const scrollPositionsRef = useRef<Record<string, number>>({})
  const scrollTopRef = useRef(0)
  const showScrollTopRef = useRef(false)
  const restoredScrollKeyRef = useRef<string | null>(null)
  const [showScrollTop, setShowScrollTop] = useState(false)

  const scrollPositionKey = useMemo(
    () => buildScrollPositionKey(mode, isChainMode, activeSelectedGroup),
    [activeSelectedGroup, isChainMode, mode],
  )

  const saveScrollPosition = useCallback(
    (scrollTop: number) => {
      scrollPositionsRef.current[scrollPositionKey] = scrollTop
      saveStoredScrollPositions(scrollPositionsRef.current)
    },
    [scrollPositionKey],
  )

  const saveScrollPositionThrottled = useMemo(
    () => throttle(saveScrollPosition, 500),
    [saveScrollPosition],
  )

  const syncScrollTopVisibility = useCallback((scrollTop: number) => {
    const nextShowScrollTop = scrollTop > SCROLL_TOP_VISIBILITY_THRESHOLD

    if (showScrollTopRef.current !== nextShowScrollTop) {
      showScrollTopRef.current = nextShowScrollTop
      setShowScrollTop(nextShowScrollTop)
    }
  }, [])

  const handleScroll = useCallback(
    (event: Event) => {
      const target = event.target as HTMLElement | null
      const nextScrollTop = target?.scrollTop ?? 0

      scrollTopRef.current = nextScrollTop
      syncScrollTopVisibility(nextScrollTop)
      saveScrollPositionThrottled(nextScrollTop)
    },
    [saveScrollPositionThrottled, syncScrollTopVisibility],
  )

  const restoreScrollPosition = useCallback(() => {
    const parentElement = parentRef.current
    if (renderListLength === 0 || !parentElement) return

    if (
      restoredScrollKeyRef.current === scrollPositionKey &&
      parentElement.scrollTop === scrollTopRef.current
    ) {
      return
    }

    const savedPositions = loadStoredScrollPositions()
    scrollPositionsRef.current = savedPositions

    const savedPosition = savedPositions[scrollPositionKey]
    if (savedPosition === undefined) {
      restoredScrollKeyRef.current = scrollPositionKey
      return
    }

    parentElement.scrollTop = savedPosition
    scrollTopRef.current = savedPosition
    showScrollTopRef.current =
      savedPosition > SCROLL_TOP_VISIBILITY_THRESHOLD
    queueMicrotask(() => syncScrollTopVisibility(savedPosition))

    restoredScrollKeyRef.current = scrollPositionKey
  }, [renderListLength, scrollPositionKey, syncScrollTopVisibility])

  useLayoutEffect(() => {
    restoreScrollPosition()
  }, [pathname, restoreScrollPosition])

  useEffect(() => {
    const node = parentRef.current
    if (!node) return

    const restoredScrollKey = restoredScrollKeyRef.current
    const savedScrollTop = scrollTopRef.current
    const listener = handleScroll as EventListener
    const listenerOptions: AddEventListenerOptions = { passive: true }

    node.addEventListener('scroll', listener, listenerOptions)

    return () => {
      if (restoredScrollKey === scrollPositionKey) {
        saveScrollPosition(savedScrollTop)
      }

      node.removeEventListener('scroll', listener, listenerOptions)
    }
  }, [handleScroll, saveScrollPosition, scrollPositionKey])

  const scrollToTop = useCallback(() => {
    parentRef.current?.scrollTo?.({
      top: 0,
      behavior: 'smooth',
    })
    scrollTopRef.current = 0
    saveScrollPosition(0)
    syncScrollTopVisibility(0)
  }, [saveScrollPosition, syncScrollTopVisibility])

  return {
    parentRef,
    scrollToTop,
    showScrollTop,
  }
}
