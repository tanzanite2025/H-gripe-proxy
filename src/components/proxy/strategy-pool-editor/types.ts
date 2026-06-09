import type { StrategyPoolGroupRef } from '../strategy-pools/types'

export type EditableStrategyGroupState = {
  baseGroup: IProxyGroupConfig
}

export type GroupSequence = {
  prepend: IProxyGroupConfig[]
  append: IProxyGroupConfig[]
  delete: string[]
}

export type StrategyGroupLoadWarning = 'configNotReady' | 'groupsReadFailed'

export type EditableStrategyGroupLoadResult = {
  sequence: GroupSequence
  state: EditableStrategyGroupState
  selectedNames: string[]
  warnings: StrategyGroupLoadWarning[]
}

export type CandidateOption = {
  name: string
  type: string
  provider?: string
  isGroup: boolean
}

export interface UseStrategyPoolEditorOptions {
  open: boolean
  group: StrategyPoolGroupRef | null
  onClose: () => void
  onSaved?: () => Promise<void> | void
}
