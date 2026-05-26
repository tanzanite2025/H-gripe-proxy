import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react'

import { useTrafficGraphDataEnhanced } from '@/hooks/network'

import type { TimeRange } from '../utils/graph-config'
import { TARGET_FPS, STALE_DATA_THRESHOLD } from '../utils/graph-config'
import { computeYScale, isSameTrafficData } from '../utils/graph-helpers'

/**
 * 数据 reducer
 */
const displayDataReducer = (
  current: ITrafficDataPoint[],
  payload: ITrafficDataPoint[],
): ITrafficDataPoint[] =>
  isSameTrafficData(current, payload) ? current : payload

/**
 * 图表数据管理 Hook
 * 处理数据获取、缓存、防抖等
 */
export const useGraphData = (timeRange: TimeRange) => {
  // 使用增强版全局流量数据管理
  const { dataPoints, requestRange, samplerStats } =
    useTrafficGraphDataEnhanced()

  // 当前显示的数据缓存
  const [displayData, dispatchDisplayData] = useReducer(
    displayDataReducer,
    [],
  )
  const debounceTimeoutRef = useRef<number | null>(null)

  // 数据状态追踪
  const lastDataTimestampRef = useRef<number>(0)
  const dataStaleRef = useRef<boolean>(false)
  const [currentFPS, setCurrentFPS] = useState(TARGET_FPS)

  // 更新显示数据（防抖处理）
  const updateDisplayData = useCallback((newData: ITrafficDataPoint[]) => {
    if (debounceTimeoutRef.current !== null) {
      window.clearTimeout(debounceTimeoutRef.current)
    }
    debounceTimeoutRef.current = window.setTimeout(() => {
      dispatchDisplayData(newData)
    }, 50) // 50ms防抖
  }, [])

  // 监听数据变化
  useEffect(() => {
    updateDisplayData(dataPoints)

    return () => {
      if (debounceTimeoutRef.current !== null) {
        window.clearTimeout(debounceTimeoutRef.current)
        debounceTimeoutRef.current = null
      }
    }
  }, [dataPoints, updateDisplayData])

  // 请求时间范围数据
  useEffect(() => {
    requestRange(timeRange)
  }, [requestRange, timeRange])

  // 更新数据状态
  useEffect(() => {
    if (displayData.length === 0) {
      lastDataTimestampRef.current = 0
      dataStaleRef.current = false
      setCurrentFPS(TARGET_FPS)
      return
    }

    const latestTimestamp =
      displayData[displayData.length - 1]?.timestamp ?? null
    if (latestTimestamp) {
      lastDataTimestampRef.current = latestTimestamp
      const age = Date.now() - latestTimestamp
      const stale = age > STALE_DATA_THRESHOLD
      dataStaleRef.current = stale
    } else {
      dataStaleRef.current = false
    }
  }, [displayData])

  // 计算Y轴刻度
  const yScale = useMemo(() => computeYScale(displayData), [displayData])

  return {
    displayData,
    yScale,
    samplerStats,
    currentFPS,
    setCurrentFPS,
    lastDataTimestampRef,
    dataStaleRef,
  }
}
