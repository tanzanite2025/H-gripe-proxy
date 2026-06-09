import yaml from 'js-yaml'

import { readProfileFile } from '@/services/cmds'

import { parseGroupsYaml } from '../../profile/groups-editor-viewer/utils/group-helpers'

const MANAGED_STRATEGY_GROUP_TYPES = new Set(['url-test', 'load-balance'])

export const normalizeNames = (names: Array<string | null | undefined>) =>
  Array.from(
    new Set(
      names
        .map((name) => name?.trim() || '')
        .filter((name) => name.length > 0),
    ),
  )

export const isManagedStrategyGroupConfig = (
  group?: IProxyGroupConfig | null,
) => {
  const name = group?.name?.trim() || ''
  const type = group?.type?.trim().toLowerCase() || ''
  return Boolean(name) && MANAGED_STRATEGY_GROUP_TYPES.has(type)
}

export const readProfileProxyGroups = async (profileUid: string) => {
  const profileData = await readProfileFile(profileUid)
  const profileObject = yaml.load(profileData) as
    | { 'proxy-groups'?: IProxyGroupConfig[] }
    | null

  return profileObject?.['proxy-groups'] || []
}

export const readOverrideGroups = async (groupsOverridePath: string) => {
  const groupsData = await readProfileFile(groupsOverridePath)
  const sequence = parseGroupsYaml(groupsData)

  return [...sequence.prepend, ...sequence.append] as IProxyGroupConfig[]
}
