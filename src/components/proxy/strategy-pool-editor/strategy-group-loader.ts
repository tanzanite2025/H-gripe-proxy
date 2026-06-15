import { readProfileFile } from '@/services/cmds'

import { parseGroupsYaml } from '../../profile/groups-editor-viewer/utils/group-helpers'
import { filterStrategyPoolMemberNames } from '../strategy-pools/strategy-pool-rules'
import type { StrategyPoolGroupRef } from '../strategy-pools/types'

import {
  buildFallbackGroupConfig,
  cloneGroupConfig,
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
  group: StrategyPoolGroupRef,
  property?: string,
): Promise<EditableStrategyGroupLoadResult> => {
  const warnings: StrategyGroupLoadWarning[] = []
  let sequence: GroupSequence = EMPTY_GROUP_SEQUENCE

  if (property) {
    try {
      const groupsData = await readProfileFile(property)
      sequence = parseGroupsYaml(groupsData)
    } catch {
      warnings.push('groupsReadFailed')
    }
  } else {
    warnings.push('configNotReady')
  }

  const overrideGroup =
    findLastGroupByName(sequence.append, group.name) ||
    findLastGroupByName(sequence.prepend, group.name)
  const baseGroup = cloneGroupConfig(
    overrideGroup || buildFallbackGroupConfig(group),
  )
  const selectedNames = filterStrategyPoolMemberNames(overrideGroup?.proxies)

  return {
    sequence,
    state: {
      baseGroup,
    },
    selectedNames,
    warnings,
  }
}
