import { useCallback } from 'react'

// 本地存储键
export const STORAGE_KEY_GROUP = 'clash-verge-selected-proxy-group'
export const STORAGE_KEY_PROXY = 'clash-verge-selected-proxy'
export const STORAGE_KEY_SORT_TYPE = 'clash-verge-proxy-sort-type'

// 处理按配置文件隔离的代理选择持久化。
export const useProxyStorage = (currentProfileId: string | null) => {
  const getProfileStorageKey = useCallback(
    (baseKey: string) =>
      currentProfileId ? `${baseKey}:${currentProfileId}` : baseKey,
    [currentProfileId],
  )

  const readProfileScopedItem = useCallback(
    (baseKey: string): string | null => {
      if (typeof window === 'undefined') return null

      const profileKey = getProfileStorageKey(baseKey)
      const profileValue = localStorage.getItem(profileKey)

      if (profileValue != null) {
        return profileValue
      }

      // 迁移旧的全局值到当前配置文件作用域。
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

  const writeProfileScopedItem = useCallback(
    (baseKey: string, value: string) => {
      if (typeof window === 'undefined') return

      const profileKey = getProfileStorageKey(baseKey)
      localStorage.setItem(profileKey, value)

      // 清理旧的全局值。
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
