import { useQuery } from '@tanstack/react-query'
import { useCallback, useMemo } from 'react'

import { useProxySelection } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import { useProxiesData } from '@/providers/app-data-context'
import { calcuProxies } from '@/services/cmds'

import { useRenderList } from '../../use-render-list'

interface UseProxyGroupsOptions {
  mode: string
  isChainMode: boolean
  activeSelectedGroup: string | null
}

/**
 * 管理代理组数据和业务逻辑
 */
export function useProxyGroups(options: UseProxyGroupsOptions) {
  const { mode, isChainMode, activeSelectedGroup } = options

  const { verge } = useVerge()
  const { proxies: proxiesData } = useProxiesData()

  // 轮询获取代理数据（3秒间隔）
  useQuery({
    queryKey: ['getProxies'],
    queryFn: calcuProxies,
    refetchInterval: 3000,
    refetchIntervalInBackground: false,
    staleTime: 1500,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  })

  // 获取渲染列表
  const { renderList, onProxies, onHeadState } = useRenderList(
    mode,
    isChainMode,
    activeSelectedGroup,
  )

  // 获取代理组的头部状态
  const getGroupHeadState = useCallback(
    (groupName: string) => {
      const headItem = renderList.find(
        (item) => item.type === 1 && item.group?.name === groupName,
      )
      return headItem?.headState
    },
    [renderList],
  )

  // 代理选择处理
  const { handleProxyGroupChange } = useProxySelection({
    onSuccess: () => {
      onProxies()
    },
    onError: (error) => {
      console.error('代理切换失败', error)
      onProxies()
    },
  })

  // 延迟测试超时时间
  const timeout = verge?.default_latency_timeout || 10000

  // 代理组名称列表（用于导航）
  const proxyGroupNames = useMemo(() => {
    const names = renderList
      .filter((item) => item.type === 0 && item.group?.name)
      .map((item) => item.group!.name)
    return Array.from(new Set(names))
  }, [renderList])

  // 定位到指定的代理节点
  const handleLocation = useCallback(
    (group: IProxyGroupItem, scrollToIndex: (index: number, options?: any) => void) => {
      if (!group) return
      const { name, now } = group

      const index = renderList.findIndex(
        (e) =>
          e.group?.name === name &&
          ((e.type === 2 && e.proxy?.name === now) ||
            (e.type === 4 && e.proxyCol?.some((p) => p.name === now))),
      )

      if (index >= 0) {
        scrollToIndex(index, { align: 'center', behavior: 'smooth' })
      }
    },
    [renderList],
  )

  // 定位到指定的代理组
  const handleGroupLocationByName = useCallback(
    (groupName: string, scrollToIndex: (index: number, options?: any) => void) => {
      const index = renderList.findIndex(
        (item) => item.type === 0 && item.group?.name === groupName,
      )

      if (index >= 0) {
        scrollToIndex(index, { align: 'start', behavior: 'smooth' })
      }
    },
    [renderList],
  )

  return {
    // 数据
    proxiesData,
    renderList,
    timeout,
    proxyGroupNames,
    verge,

    // 方法
    onProxies,
    onHeadState,
    getGroupHeadState,
    handleProxyGroupChange,
    handleLocation,
    handleGroupLocationByName,
  }
}
