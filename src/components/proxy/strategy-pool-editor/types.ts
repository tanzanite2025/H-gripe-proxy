export type EditableStrategyGroupState = {
  baseGroup: IProxyGroupConfig
  originExists: boolean
}

export type GroupSequence = {
  prepend: IProxyGroupConfig[]
  append: IProxyGroupConfig[]
  delete: string[]
}

export type StrategyGroupLoadWarning =
  | 'profileNotReady'
  | 'profileReadFailed'
  | 'groupsReadFailed'

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
  group: IProxyGroupItem | null
  onClose: () => void
  onSaved?: () => Promise<void> | void
}
