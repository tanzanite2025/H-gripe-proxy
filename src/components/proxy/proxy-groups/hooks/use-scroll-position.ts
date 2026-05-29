import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { useLocation } from 'react-router'

import { throttle } from '../utils/helpers'

interface UseScrollPositionOptions {
  mode: string
  isChainMode: boolean
  activeSelectedGroup: string | null
  renderListLength: number
}

/**
 * 管理滚动位置的持久化
 */
export function useScrollPosition(options: UseScrollPositionOptions) {
  const { mode, isChainMode, activeSelectedGroup, renderListLength } = options
  const { pathname } = useLocation()

  const scrollPositionRef = useRef<Record<string, number>>({})
  const scrollTopRef = useRef(0)
  const showScrollTopRef = useRef(false)
  const restoredScrollKeyRef = useRef<string | null>(null)
  const [showScrollTop, setShowScrollTop] = useState(false)

  // 生成滚动位置的唯一键
  const scrollPositionKey = useMemo(
    () =>
      isChainMode
        ? `${mode}:chain:${activeSelectedGroup ?? 'all'}`
        : `${mode}:normal`,
    [activeSelectedGroup, isChainMode, mode],
  )

  // 保存滚动位置到 localStorage
  const saveScrollPosition = useCallback(
    (scrollTop: number) => {
      try {
        scrollPositionRef.current[scrollPositionKey] = scrollTop
        localStorage.setItem(
          'proxy-scroll-positions',
          JSON.stringify(scrollPositionRef.current),
        )
      } catch (e) {
        console.error('Error saving scroll position:', e)
      }
    },
    [scrollPositionKey],
  )

  // 节流保存滚动位置
  const saveScrollPositionThrottled = useMemo(
    () => throttle(saveScrollPosition, 500),
    [saveScrollPosition],
  )

  // 处理滚动事件
  const handleScroll = useCallback(
    (event: Event) => {
      const target = event.target as HTMLElement | null
      const nextScrollTop = target?.scrollTop ?? 0
      const nextShowScrollTop = nextScrollTop > 100
      scrollTopRef.current = nextScrollTop

      if (showScrollTopRef.current !== nextShowScrollTop) {
        showScrollTopRef.current = nextShowScrollTop
        setShowScrollTop(nextShowScrollTop)
      }

      saveScrollPositionThrottled(nextScrollTop)
    },
    [saveScrollPositionThrottled],
  )

  // 从 localStorage 恢复滚动位置
  const restoreScrollPosition = useCallback(
    (parentElement: HTMLDivElement | null) => {
      if (renderListLength === 0) return
      if (!parentElement) return
      if (
        restoredScrollKeyRef.current === scrollPositionKey &&
        parentElement.scrollTop === scrollTopRef.current
      ) {
        return
      }

      try {
        const savedPositions = localStorage.getItem('proxy-scroll-positions')
        if (savedPositions) {
          const positions = JSON.parse(savedPositions)
          scrollPositionRef.current = positions
          const savedPosition = positions[scrollPositionKey]

          if (savedPosition !== undefined) {
            parentElement.scrollTop = savedPosition
            scrollTopRef.current = savedPosition
            const nextShowScrollTop = savedPosition > 100
            showScrollTopRef.current = nextShowScrollTop
            queueMicrotask(() => setShowScrollTop(nextShowScrollTop))
          }
        }
      } catch (e) {
        console.error('Error restoring scroll position:', e)
      }
      restoredScrollKeyRef.current = scrollPositionKey
    },
    [renderListLength, scrollPositionKey],
  )

  // 滚动到顶部
  const scrollToTop = useCallback(
    (parentElement: HTMLDivElement | null) => {
      parentElement?.scrollTo?.({
        top: 0,
        behavior: 'smooth',
      })
      scrollTopRef.current = 0
      saveScrollPosition(0)
    },
    [saveScrollPosition],
  )

  return {
    showScrollTop,
    scrollPositionKey,
    handleScroll,
    restoreScrollPosition,
    scrollToTop,
    saveScrollPosition,
  }
}

/**
 * 管理滚动事件监听器
 */
export function useScrollListener(
  parentRef: React.RefObject<HTMLDivElement | null>,
  handleScroll: (event: Event) => void,
  saveScrollPosition: (scrollTop: number) => void,
  scrollPositionKey: string,
  scrollTopRef: React.MutableRefObject<number>,
  restoredScrollKeyRef: React.MutableRefObject<string | null>,
) {
  useEffect(() => {
    const node = parentRef.current
    if (!node) return
    const restoredScrollKey = restoredScrollKeyRef.current
    const savedScrollTop = scrollTopRef.current

    const listener = handleScroll as EventListener
    const options: AddEventListenerOptions = { passive: true }

    node.addEventListener('scroll', listener, options)

    return () => {
      if (restoredScrollKey === scrollPositionKey) {
        saveScrollPosition(savedScrollTop)
      }
      node.removeEventListener('scroll', listener, options)
    }
  }, [
    handleScroll,
    parentRef,
    restoredScrollKeyRef,
    saveScrollPosition,
    scrollPositionKey,
    scrollTopRef,
  ])
}

/**
 * 恢复滚动位置的 Layout Effect
 */
export function useRestoreScrollPosition(
  parentRef: React.RefObject<HTMLDivElement | null>,
  restoreScrollPosition: (element: HTMLDivElement | null) => void,
  pathname: string,
) {
  useLayoutEffect(() => {
    restoreScrollPosition(parentRef.current)
  }, [pathname, parentRef, restoreScrollPosition])
}
