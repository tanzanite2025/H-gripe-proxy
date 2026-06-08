import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'

import {
  useProxiesData,
  useRulesData,
} from '@/providers/app-data-context'
import { getCurrentEgressIdentity } from '@/services/cmds/diagnostics'
import {
  analyzeProxyPath,
  getIdentityProxyChain,
  getPreferredProxyGroupName,
  resolveProxyPath,
} from '@/services/proxy-display'

import { buildDisplayPath, type IRenderItem } from './render-list-shared'

export const useRuntimeSummaryItem = (mode: string): IRenderItem | null => {
  const { proxies: proxiesData } = useProxiesData()
  const { rules } = useRulesData()
  const { data: currentIdentity } = useQuery({
    queryKey: ['current-egress-identity'],
    queryFn: getCurrentEgressIdentity,
    staleTime: 5_000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: true,
    refetchInterval: 5_000,
    retry: 1,
  })

  return useMemo(() => {
    if (!proxiesData) return null

    const identityPath = analyzeProxyPath(
      getIdentityProxyChain(currentIdentity),
      proxiesData.records,
    )
    const fallbackRootGroup = getPreferredProxyGroupName({
      proxies: proxiesData,
      rules,
      isGlobalMode: mode === 'global',
    })
    const fallbackPath = resolveProxyPath(proxiesData.records, fallbackRootGroup)
    const activePath = identityPath.path.length ? identityPath : fallbackPath
    const activePathDisplay = buildDisplayPath(
      activePath.path,
      proxiesData.records,
    )

    if (!activePathDisplay.length) {
      return null
    }

    return {
      type: 5,
      key: 'runtime-summary',
      sectionKind: 'runtime',
      sectionTitle:
        identityPath.path.length > 0 ? '当前实际链路' : '当前链路参考',
      sectionDescription:
        identityPath.path.length > 0
          ? currentIdentity?.rule
            ? `匹配规则: ${currentIdentity.rule}`
            : 'Mihomo 已返回当前实际链路。'
          : fallbackRootGroup
            ? `Mihomo 尚未返回实际链路，当前按 ${fallbackRootGroup} 的运行态解析显示。`
            : 'Mihomo 尚未返回实际链路，当前按本地运行态解析显示。',
      runtimePath: activePathDisplay,
      runtimeObserved: identityPath.path.length > 0,
      runtimeDescription:
        identityPath.path.length > 0
          ? currentIdentity?.proxy_name
            ? `最终节点: ${currentIdentity.proxy_name}`
            : '已观测到实际链路。'
          : '仅供当前页面参考，不代表某一条正在连接的业务流。',
    } satisfies IRenderItem
  }, [currentIdentity, mode, proxiesData, rules])
}
