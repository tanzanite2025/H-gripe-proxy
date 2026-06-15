import { buildGroupsYaml } from '../../profile/groups-editor-viewer/utils/group-helpers'
import { normalizeStrategyPoolNames } from '../strategy-pools/strategy-pool-rules'
import type { StrategyPoolGroupRef } from '../strategy-pools/types'

import type { GroupSequence } from './types'

export const cloneGroupConfig = (
  group: IProxyGroupConfig,
): IProxyGroupConfig => ({
  ...group,
  proxies: Array.isArray(group.proxies) ? [...group.proxies] : undefined,
  use: Array.isArray(group.use) ? [...group.use] : undefined,
})

export const buildFallbackGroupConfig = (
  group: StrategyPoolGroupRef,
): IProxyGroupConfig => ({
  name: group.name,
  type: group.configType,
  proxies: [],
  url: group.testUrl,
  hidden: group.hidden,
  icon: group.icon,
  'interface-name': '',
})

export const buildStrategyGroupYaml = (
  groupName: string,
  sequence: GroupSequence,
  nextGroup: IProxyGroupConfig,
) => {
  const nextPrepend = sequence.prepend.filter((item) => item?.name !== groupName)
  const nextAppend = sequence.append.filter((item) => item?.name !== groupName)
  const nextDelete = sequence.delete.filter((name) => name !== groupName)

  return buildGroupsYaml(
    nextPrepend,
    [...nextAppend, nextGroup],
    normalizeStrategyPoolNames(nextDelete),
  )
}
