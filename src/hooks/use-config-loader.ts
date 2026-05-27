/**
 * 通用配置加载 Hook
 * 
 * 提供统一的配置加载、错误处理、加载状态管理
 */

import { useState, useEffect, useCallback } from 'react'
import { showNotice } from '@/services/notice-service'

export interface UseConfigLoaderOptions<T> {
  /** 加载函数 */
  loadFn: () => Promise<T>
  /** 加载成功回调 */
  onSuccess?: (data: T) => void
  /** 加载失败回调 */
  onError?: (error: Error) => void
  /** 是否自动加载（默认 true） */
  autoLoad?: boolean
  /** 是否显示错误通知（默认 true） */
  showErrorNotice?: boolean
}

export interface UseConfigLoaderResult<T> {
  /** 加载的数据 */
  data: T | null
  /** 是否正在加载 */
  loading: boolean
  /** 错误信息 */
  error: Error | null
  /** 加载函数 */
  load: () => Promise<T | null>
  /** 重新加载（别名） */
  reload: () => Promise<T | null>
}

/**
 * 通用配置加载 Hook
 * 
 * @example
 * ```typescript
 * // 基础用法
 * const { data, loading, reload } = useConfigLoader({
 *   loadFn: getAdvancedConfig,
 * })
 * 
 * // 带回调
 * const { data, loading } = useConfigLoader({
 *   loadFn: getAdvancedConfig,
 *   onSuccess: (config) => console.log('加载成功', config),
 *   onError: (error) => console.error('加载失败', error),
 * })
 * 
 * // 手动加载
 * const { data, load } = useConfigLoader({
 *   loadFn: getAdvancedConfig,
 *   autoLoad: false,
 * })
 * ```
 */
export function useConfigLoader<T>(
  options: UseConfigLoaderOptions<T>,
): UseConfigLoaderResult<T> {
  const {
    loadFn,
    onSuccess,
    onError,
    autoLoad = true,
    showErrorNotice = true,
  } = options

  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  const load = useCallback(async (): Promise<T | null> => {
    try {
      setLoading(true)
      setError(null)
      const result = await loadFn()
      setData(result)
      onSuccess?.(result)
      return result
    } catch (err: any) {
      const error = err instanceof Error ? err : new Error(err.toString())
      setError(error)
      onError?.(error)
      if (showErrorNotice) {
        showNotice('error', error.message || '加载配置失败')
      }
      return null
    } finally {
      setLoading(false)
    }
  }, [loadFn, onSuccess, onError, showErrorNotice])

  // 自动加载
  useEffect(() => {
    if (autoLoad) {
      load()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return {
    data,
    loading,
    error,
    load,
    reload: load,
  }
}

/**
 * 加载多个配置
 * 
 * @example
 * ```typescript
 * const { data, loading } = useMultiConfigLoader({
 *   loaders: {
 *     config: getAdvancedConfig,
 *     status: coordinatorGetStatus,
 *   },
 * })
 * 
 * // data 的类型为 { config: AdvancedConfig, status: CoordinatorStatus } | null
 * ```
 */
export function useMultiConfigLoader<T extends Record<string, () => Promise<any>>>(options: {
  loaders: T
  onSuccess?: (data: { [K in keyof T]: Awaited<ReturnType<T[K]>> }) => void
  onError?: (error: Error) => void
  autoLoad?: boolean
  showErrorNotice?: boolean
}) {
  const { loaders, onSuccess, onError, autoLoad = true, showErrorNotice = true } = options

  type ResultType = { [K in keyof T]: Awaited<ReturnType<T[K]>> }

  const [data, setData] = useState<ResultType | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  const load = useCallback(async (): Promise<ResultType | null> => {
    try {
      setLoading(true)
      setError(null)

      // 并行加载所有配置
      const keys = Object.keys(loaders) as Array<keyof T>
      const promises = keys.map((key) => loaders[key]())
      const results = await Promise.all(promises)

      // 构建结果对象
      const resultObj = {} as ResultType
      keys.forEach((key, index) => {
        resultObj[key] = results[index]
      })

      setData(resultObj)
      onSuccess?.(resultObj)
      return resultObj
    } catch (err: any) {
      const error = err instanceof Error ? err : new Error(err.toString())
      setError(error)
      onError?.(error)
      if (showErrorNotice) {
        showNotice('error', error.message || '加载配置失败')
      }
      return null
    } finally {
      setLoading(false)
    }
  }, [loaders, onSuccess, onError, showErrorNotice])

  // 自动加载
  useEffect(() => {
    if (autoLoad) {
      load()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return {
    data,
    loading,
    error,
    load,
    reload: load,
  }
}
