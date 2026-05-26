import { useCallback } from 'react'

// 本地存储的键名
export const STORAGE_KEY_GROUP = 'clash-verge-selected-proxy-group'
export const STORAGE_KEY_PROXY = 'clash-verge-selected-proxy'
export const STORAGE_KEY_SORT_TYPE = 'clash-verge-proxy-sort-type'

/**
 * 代理存储管理 Hook
 * 处理 localStorage 的读写，支持按配置文件隔离
 */
export const useProxyStorage = (currentProfileId: string | null) => {
  /**
   * 获取带配置文件前缀的存储键
   */
  const getProfileStorageKey = useCallback(
    (baseKey: string) =>
      currentProfileId ? `${baseKey}:${currentProfileId}` : baseKey,
    [currentProfileId],
  )

  /**
   * 读取配置文件作用域的存储项
   * 优先读取带配置文件前缀的值，如果不存在则读取旧的全局值并迁移
   */
  const readProfileScopedItem = useCallback(
    (baseKey: string): string | null => {
      if (typeof window === 'undefined') return null

      const profileKey = getProfileStorageKey(baseKey)
      const profileValue = localStorage.getItem(profileKey)

      if (profileValue != null) {
        return profileValue
      }

      // 尝试迁移旧的全局值
      if (profileKey !== baseKey) {
        const legacyValue = localStorage.getItem(baseKey)
        if (legacyValue != null) {
          localStorage.removeItem(baseKey)
          localStorage.setItem(profileKey, legacyValue)
          return legacyValue
        }
      }

      return null
    },
    [getProfileStorageKey],
  )

  /**
   * 写入配置文件作用域的存储项
   * 同时清理旧的全局值
   */
  const writeProfileScopedItem = useCallback(
    (baseKey: string, value: string) => {
      if (typeof window === 'undefined') return

      const profileKey = getProfileStorageKey(baseKey)
      localStorage.setItem(profileKey, value)

      // 清理旧的全局值
      if (profileKey !== baseKey) {
        localStorage.removeItem(baseKey)
      }
    },
    [getProfileStorageKey],
  )

  return {
    readProfileScopedItem,
    writeProfileScopedItem,
  }
}
