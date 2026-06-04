import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useProxiesData } from '@/providers/app-data-context'

import { clearProxyChainRuntimeConfig } from '../../proxy-chain-runtime'
import {
  clearProxyChainStorage,
  loadProxyChainStorage,
  saveProxyChainStorage,
  type ProxyChainItem,
} from '../../proxy-chain-types'

interface UseChainModeOptions {
  isChainMode: boolean
  mode: string
}

/**
 * 管理链式代理模式的状态和逻辑
 */
export function useChainMode(options: UseChainModeOptions) {
  const { isChainMode, mode } = options
  const { t } = useTranslation()
  const { proxies: proxiesData } = useProxiesData()

  // 代理链状态
  const [proxyChain, setProxyChain] = useState<ProxyChainItem[]>(
    loadProxyChainStorage,
  )

  // 选中的代理组
  const [selectedGroup, setSelectedGroup] = useState<string | null>(null)

  // 代理组选择菜单
  const [ruleMenuAnchor, setRuleMenuAnchor] = useState<null | HTMLElement>(null)

  // 重复节点警告
  const [duplicateWarning, setDuplicateWarning] = useState<{
    open: boolean
    message: string
  }>({ open: false, message: '' })

  // 持久化代理链到 localStorage
  useEffect(() => {
    saveProxyChainStorage(proxyChain)
  }, [proxyChain])

  // 获取可用的代理组（链式模式下只显示 Selector 类型）
  const groups = proxiesData?.groups
  const availableGroups = useMemo(() => {
    if (!groups) return []
    return isChainMode
      ? groups.filter((g: any) => g.type === 'Selector')
      : groups
  }, [groups, isChainMode])

  // 默认规则组
  const defaultRuleGroup = useMemo(() => {
    if (isChainMode && mode === 'rule' && availableGroups.length > 0) {
      return availableGroups[0].name
    }
    return null
  }, [availableGroups, isChainMode, mode])

  // 当前激活的代理组
  const activeSelectedGroup = useMemo(
    () => selectedGroup ?? defaultRuleGroup,
    [selectedGroup, defaultRuleGroup],
  )

  // 当前代理组对象
  const currentGroup = useMemo(() => {
    if (!activeSelectedGroup) return null
    return (
      availableGroups.find(
        (group: any) => group.name === activeSelectedGroup,
      ) ?? null
    )
  }, [activeSelectedGroup, availableGroups])

  // 打开代理组选择菜单
  const handleGroupMenuOpen = useCallback((event: React.MouseEvent<HTMLElement>) => {
    setRuleMenuAnchor(event.currentTarget)
  }, [])

  // 关闭代理组选择菜单
  const handleGroupMenuClose = useCallback(() => {
    setRuleMenuAnchor(null)
  }, [])

  // 选择代理组
  const handleGroupSelect = useCallback(
    (groupName: string) => {
      setSelectedGroup(groupName)
      handleGroupMenuClose()

      if (isChainMode && mode === 'rule') {
        clearProxyChainRuntimeConfig()
        clearProxyChainStorage()
        setProxyChain([])
      }
    },
    [handleGroupMenuClose, isChainMode, mode],
  )

  // 添加代理到链
  const addProxyToChain = useCallback(
    (proxy: IProxyItem) => {
      setProxyChain((prev) => {
        // 检查是否已经存在相同名称的代理
        if (prev.some((item) => item.name === proxy.name)) {
          const warningMessage = t('proxies.page.chain.duplicateNode')
          setDuplicateWarning({
            open: true,
            message: warningMessage,
          })
          return prev
        }

        // 获取延迟数据
        const delay =
          proxy.history && proxy.history.length > 0
            ? proxy.history[proxy.history.length - 1].delay
            : undefined

        const chainItem: ProxyChainItem = {
          id: `${proxy.name}_${Date.now()}`,
          name: proxy.name,
          type: proxy.type,
          delay: delay,
        }

        return [...prev, chainItem]
      })
    },
    [t],
  )

  // 关闭重复节点警告
  const handleCloseDuplicateWarning = useCallback(() => {
    setDuplicateWarning({ open: false, message: '' })
  }, [])

  return {
    // 状态
    proxyChain,
    selectedGroup,
    ruleMenuAnchor,
    duplicateWarning,
    availableGroups,
    activeSelectedGroup,
    currentGroup,

    // 方法
    setProxyChain,
    handleGroupMenuOpen,
    handleGroupMenuClose,
    handleGroupSelect,
    addProxyToChain,
    handleCloseDuplicateWarning,
  }
}
