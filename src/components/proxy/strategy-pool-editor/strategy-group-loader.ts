import yaml from 'js-yaml'

import { readProfileFile } from '@/services/cmds'
import { isBuiltinPolicyName, isHiddenProxyName } from '@/services/proxy-display'

import { parseGroupsYaml } from '../../profile/groups-editor-viewer/utils/group-helpers'
import {
  buildFallbackGroupConfig,
  cloneGroupConfig,
  normalizeNames,
} from './group-config'
import type {
  EditableStrategyGroupLoadResult,
  GroupSequence,
  StrategyGroupLoadWarning,
} from './types'

const EMPTY_GROUP_SEQUENCE: GroupSequence = {
  prepend: [],
  append: [],
  delete: [],
}

const findLastGroupByName = (
  list: IProxyGroupConfig[],
  name: string,
): IProxyGroupConfig | undefined => {
  for (let index = list.length - 1; index >= 0; index -= 1) {
    const item = list[index]
    if (item?.name === name) {
      return item
    }
  }

  return undefined
}

export const loadEditableStrategyGroup = async (
  group: IProxyGroupItem,
  profileUid?: string,
  property?: string,
): Promise<EditableStrategyGroupLoadResult> => {
  const warnings: StrategyGroupLoadWarning[] = []
  let originGroup: IProxyGroupConfig | undefined
  let sequence: GroupSequence = EMPTY_GROUP_SEQUENCE

  if (profileUid) {
    try {
      const profileData = await readProfileFile(profileUid)
      const profileObject = yaml.load(profileData) as
        | { 'proxy-groups'?: IProxyGroupConfig[] }
        | null
      const originGroups = profileObject?.['proxy-groups'] || []
      originGroup = originGroups.find((item) => item?.name === group.name)
    } catch {
      warnings.push('profileReadFailed')
    }
  }

  if (property) {
    try {
      const groupsData = await readProfileFile(property)
      sequence = parseGroupsYaml(groupsData)
    } catch {
      warnings.push('groupsReadFailed')
    }
  } else {
    warnings.push('profileNotReady')
  }

  const overrideGroup =
    findLastGroupByName(sequence.append, group.name) ||
    findLastGroupByName(sequence.prepend, group.name)
  const baseGroup = cloneGroupConfig(
    overrideGroup || originGroup || buildFallbackGroupConfig(group),
  )
  const selectedNames = normalizeNames(
    Array.isArray(overrideGroup?.proxies) ? overrideGroup.proxies : [],
  ).filter((name) => !isHiddenProxyName(name) && !isBuiltinPolicyName(name))

  return {
    sequence,
    state: {
      baseGroup,
      originExists: Boolean(originGroup),
    },
    selectedNames,
    warnings,
  }
}
