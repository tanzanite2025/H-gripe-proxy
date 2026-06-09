import { buildGroupsYaml } from '../../profile/groups-editor-viewer/utils/group-helpers'
import type { GroupSequence } from './types'

const RUNTIME_GROUP_TYPE_MAP: Record<string, IProxyGroupConfig['type']> = {
  Selector: 'select',
  URLTest: 'url-test',
  LoadBalance: 'load-balance',
  Fallback: 'fallback',
  Relay: 'relay',
}

export const normalizeNames = (names: Array<string | null | undefined>) =>
  Array.from(
    new Set(
      names
        .map((name) => name?.trim() || '')
        .filter((name) => name.length > 0),
    ),
  )

export const cloneGroupConfig = (
  group: IProxyGroupConfig,
): IProxyGroupConfig => ({
  ...group,
  proxies: Array.isArray(group.proxies) ? [...group.proxies] : undefined,
  use: Array.isArray(group.use) ? [...group.use] : undefined,
})

export const buildFallbackGroupConfig = (
  group: IProxyGroupItem,
): IProxyGroupConfig => ({
  name: group.name,
  type: RUNTIME_GROUP_TYPE_MAP[group.type] || 'url-test',
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
  originExists: boolean,
) => {
  const nextPrepend = sequence.prepend.filter((item) => item?.name !== groupName)
  const nextAppend = sequence.append.filter((item) => item?.name !== groupName)
  const nextDelete = sequence.delete.filter((name) => name !== groupName)

  if (originExists) {
    nextDelete.push(groupName)
  }

  return buildGroupsYaml(
    nextPrepend,
    [...nextAppend, nextGroup],
    normalizeNames(nextDelete),
  )
}
