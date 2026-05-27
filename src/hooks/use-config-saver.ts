/**
 * 通用配置保存 Hook
 * 
 * 提供统一的配置保存、错误处理、保存状态管理
 */

import { useState, useEffect, useCallback } from 'react'
import { showNotice } from '@/services/notice-service'

export interface UseConfigSaverOptions<T> {
  /** 保存函数 */
  saveFn: (data: T) => Promise<void>
  /** 保存成功回调 */
  onSuccess?: () => void
  /** 保存失败回调 */
  onError?: (error: Error) => void
  /** 成功提示消息（默认 "配置已保存"） */
  successMessage?: string
  /** 是否显示成功通知（默认 true） */
  showSuccessNotice?: boolean
  /** 是否显示错误通知（默认 true） */
  showErrorNotice?: boolean
}

export interface UseConfigSaverResult<T> {
  /** 保存函数 */
  save: (data: T) => Promise<boolean>
  /** 是否正在保存 */
  saving: boolean
  /** 错误信息 */
  error: Error | null
}

/**
 * 通用配置保存 Hook
 * 
 * @example
 * ```typescript
 * // 基础用法
 * const { save, saving } = useConfigSaver({
 *   saveFn: saveAdvancedConfig,
 * })
 * 
 * // 带回调和自定义消息
 * const { save, saving } = useConfigSaver({
 *   saveFn: saveAdvancedConfig,
 *   onSuccess: () => reload(),
 *   successMessage: '配置已保存并应用',
 * })
 * 
 * // 使用
 * const handleSave = () => {
 *   if (config) {
 *     save(config)
 *   }
 * }
 * ```
 */
export function useConfigSaver<T>(
  options: UseConfigSaverOptions<T>,
): UseConfigSaverResult<T> {
  const {
    saveFn,
    onSuccess,
    onError,
    successMessage = '配置已保存',
    showSuccessNotice = true,
    showErrorNotice = true,
  } = options

  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  const save = useCallback(async (data: T): Promise<boolean> => {
    try {
      setSaving(true)
      setError(null)
      await saveFn(data)
      if (showSuccessNotice) {
        showNotice('success', successMessage)
      }
      onSuccess?.()
      return true
    } catch (err: any) {
      const error = err instanceof Error ? err : new Error(err.toString())
      setError(error)
      onError?.(error)
      if (showErrorNotice) {
        showNotice('error', error.message || '保存配置失败')
      }
      return false
    } finally {
      setSaving(false)
    }
  }, [saveFn, onSuccess, onError, successMessage, showSuccessNotice, showErrorNotice])

  return {
    save,
    saving,
    error,
  }
}

/**
 * 配置加载和保存组合 Hook
 * 
 * 结合了 useConfigLoader 和 useConfigSaver 的功能
 * 
 * @example
 * ```typescript
 * const { data, loading, saving, save, reload } = useConfigManager({
 *   loadFn: getAdvancedConfig,
 *   saveFn: saveAdvancedConfig,
 * })
 * 
 * // 修改配置
 * const handleChange = (newConfig: AdvancedConfig) => {
 *   // 可以先更新本地状态
 *   setLocalConfig(newConfig)
 * }
 * 
 * // 保存配置
 * const handleSave = () => {
 *   if (localConfig) {
 *     save(localConfig)
 *   }
 * }
 * ```
 */
export function useConfigManager<T>(options: {
  loadFn: () => Promise<T>
  saveFn: (data: T) => Promise<void>
  onLoadSuccess?: (data: T) => void
  onLoadError?: (error: Error) => void
  onSaveSuccess?: () => void
  onSaveError?: (error: Error) => void
  autoLoad?: boolean
  successMessage?: string
  showSuccessNotice?: boolean
  showErrorNotice?: boolean
  /** 保存后自动重新加载（默认 true） */
  reloadAfterSave?: boolean
}) {
  const {
    loadFn,
    saveFn,
    onLoadSuccess,
    onLoadError,
    onSaveSuccess,
    onSaveError,
    autoLoad = true,
    successMessage = '配置已保存',
    showSuccessNotice = true,
    showErrorNotice = true,
    reloadAfterSave = true,
  } = options

  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  // 加载函数
  const load = useCallback(async (): Promise<T | null> => {
    try {
      setLoading(true)
      setError(null)
      const result = await loadFn()
      setData(result)
      onLoadSuccess?.(result)
      return result
    } catch (err: any) {
      const error = err instanceof Error ? err : new Error(err.toString())
      setError(error)
      onLoadError?.(error)
      if (showErrorNotice) {
        showNotice('error', error.message || '加载配置失败')
      }
      return null
    } finally {
      setLoading(false)
    }
  }, [loadFn, onLoadSuccess, onLoadError, showErrorNotice])

  // 保存函数
  const save = useCallback(async (newData: T): Promise<boolean> => {
    try {
      setSaving(true)
      setError(null)
      await saveFn(newData)
      if (showSuccessNotice) {
        showNotice('success', successMessage)
      }
      onSaveSuccess?.()
      
      // 保存后重新加载
      if (reloadAfterSave) {
        await load()
      } else {
        setData(newData)
      }
      
      return true
    } catch (err: any) {
      const error = err instanceof Error ? err : new Error(err.toString())
      setError(error)
      onSaveError?.(error)
      if (showErrorNotice) {
        showNotice('error', error.message || '保存配置失败')
      }
      return false
    } finally {
      setSaving(false)
    }
  }, [saveFn, onSaveSuccess, onSaveError, successMessage, showSuccessNotice, showErrorNotice, reloadAfterSave, load])

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
    saving,
    error,
    load,
    reload: load,
    save,
    setData,
  }
}
