import { Button, Tooltip } from '@/components/tailwind'
import { useCallback, useEffect, useMemo, useRef } from 'react'

interface ProxyGroupNavigatorProps {
  proxyGroupNames: string[]
  onGroupLocation: (groupName: string) => void
  enableHoverJump?: boolean
  hoverDelay?: number
}

export const DEFAULT_HOVER_DELAY = 280

// 提取代理组名的第一个字符
const getGroupDisplayChar = (groupName: string): string => {
  if (!groupName) return '?'

  // 直接返回第一个字符，支持表情符号
  const firstChar = Array.from(groupName)[0]
  return firstChar || '?'
}

export const ProxyGroupNavigator = ({
  proxyGroupNames,
  onGroupLocation,
  enableHoverJump = true,
  hoverDelay = DEFAULT_HOVER_DELAY,
}: ProxyGroupNavigatorProps) => {
  const lastHoveredRef = useRef<string | null>(null)
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const hoverDelayMs = hoverDelay >= 0 ? hoverDelay : 0

  const clearHoverTimer = useCallback(() => {
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current)
      hoverTimerRef.current = null
    }
  }, [])

  useEffect(() => {
    if (!enableHoverJump) {
      clearHoverTimer()
      lastHoveredRef.current = null
    }
    return () => {
      clearHoverTimer()
    }
  }, [clearHoverTimer, enableHoverJump])

  const handleGroupClick = useCallback(
    (groupName: string) => {
      clearHoverTimer()
      lastHoveredRef.current = groupName
      onGroupLocation(groupName)
    },
    [clearHoverTimer, onGroupLocation],
  )

  const handleGroupHover = useCallback(
    (groupName: string) => {
      if (!enableHoverJump) return
      if (lastHoveredRef.current === groupName) return
      clearHoverTimer()
      hoverTimerRef.current = setTimeout(() => {
        hoverTimerRef.current = null
        lastHoveredRef.current = groupName
        onGroupLocation(groupName)
      }, hoverDelayMs)
    },
    [clearHoverTimer, enableHoverJump, hoverDelayMs, onGroupLocation],
  )

  const handleButtonLeave = useCallback(() => {
    clearHoverTimer()
    lastHoveredRef.current = null
  }, [clearHoverTimer])

  // 处理代理组数据，去重和排序
  const processedGroups = useMemo(() => {
    return proxyGroupNames
      .filter((name) => name && name.trim())
      .map((name) => ({
        name,
        displayChar: getGroupDisplayChar(name),
      }))
  }, [proxyGroupNames])

  if (processedGroups.length === 0) {
    return null
  }

  return (
    <div
      className="absolute right-2 top-1/2 z-10 flex max-h-[70vh] -translate-y-1/2 flex-col gap-1 overflow-y-auto rounded p-1 scrollbar-none"
      style={{
        scrollbarWidth: 'none',
        minWidth: 'auto',
      }}
    >
      {processedGroups.map(({ name, displayChar }) => (
        <Tooltip key={name} title={name} placement="left" arrow>
          <Button
            size="small"
            variant="text"
            onClick={() => handleGroupClick(name)}
            onMouseEnter={() => handleGroupHover(name)}
            onFocus={() => handleGroupHover(name)}
            onMouseLeave={handleButtonLeave}
            onBlur={handleButtonLeave}
            className="h-7 w-7 min-h-7 min-w-7 rounded p-0 text-xs font-semibold normal-case text-gray-600 hover:bg-blue-500 hover:text-white dark:text-gray-400"
          >
            {displayChar}
          </Button>
        </Tooltip>
      ))}
    </div>
  )
}
