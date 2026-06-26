import type { CurrentEgressIdentity } from '@/services/cmds/diagnostics'
import {
  analyzeProxyPath,
  getIdentityProxyChain,
  getPreferredProxyGroupName,
  resolveProxyPath,
} from '@/services/proxy-display'
import type { CalculatedProxies } from '@/services/proxy-runtime'

import type { IRenderItem } from '../render-list/types'
import { buildDisplayPath } from '../render-list/utils'

interface BuildRuntimeSummaryItemOptions {
  currentIdentity?: CurrentEgressIdentity | null
  proxiesData?: CalculatedProxies
}

const buildRuntimeSectionDescription = ({
  currentIdentity,
  fallbackRootGroup,
  observed,
}: {
  currentIdentity?: CurrentEgressIdentity | null
  fallbackRootGroup: string
  observed: boolean
}) => {
  if (observed) {
    return currentIdentity?.rule
      ? `匹配规则: ${currentIdentity.rule}`
      : '内核已返回当前实际链路。'
  }

  if (fallbackRootGroup) {
    return `内核尚未返回实际链路，当前按 ${fallbackRootGroup} 的运行态解析显示。`
  }

  return '内核尚未返回实际链路，当前按本地运行态解析显示。'
}

const buildRuntimeDetailDescription = ({
  currentIdentity,
  observed,
}: {
  currentIdentity?: CurrentEgressIdentity | null
  observed: boolean
}) => {
  if (observed) {
    return currentIdentity?.proxy_name
      ? `最终节点: ${currentIdentity.proxy_name}`
      : '已观测到实际链路。'
  }

  return '这里仅做当前页面参考，不代表某一条正在连接中的具体业务流。'
}

export const buildRuntimeSummaryItem = ({
  currentIdentity,
  proxiesData,
}: BuildRuntimeSummaryItemOptions): IRenderItem | null => {
  if (!proxiesData) {
    return null
  }

  const identityPath = analyzeProxyPath(
    getIdentityProxyChain(currentIdentity),
    proxiesData.records,
  )
  const fallbackRootGroup = getPreferredProxyGroupName({ proxies: proxiesData })
  const fallbackPath = resolveProxyPath(proxiesData.records, fallbackRootGroup)
  const activePath = identityPath.path.length ? identityPath : fallbackPath
  const activePathDisplay = buildDisplayPath(activePath.path, proxiesData.records)
  const observed = identityPath.path.length > 0

  if (!activePathDisplay.length) {
    return null
  }

  return {
    type: 5,
    key: 'runtime-summary',
    sectionKind: 'runtime',
    sectionTitle: observed ? '当前实际链路' : '当前链路参考',
    sectionDescription: buildRuntimeSectionDescription({
      currentIdentity,
      fallbackRootGroup,
      observed,
    }),
    runtimePath: activePathDisplay,
    runtimeObserved: observed,
    runtimeDescription: buildRuntimeDetailDescription({
      currentIdentity,
      observed,
    }),
  } satisfies IRenderItem
}
