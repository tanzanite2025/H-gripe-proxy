import { StrategyPoolEditorDialogView } from './strategy-pool-editor/strategy-pool-editor-dialog-view'
import { useStrategyPoolEditor } from './strategy-pool-editor/use-strategy-pool-editor'
import type { StrategyPoolGroupRef } from './strategy-pools/types'

interface Props {
  open: boolean
  group: StrategyPoolGroupRef | null
  onClose: () => void
  onSaved?: () => Promise<void> | void
}

export function StrategyPoolEditorDialog({
  open,
  group,
  onClose,
  onSaved,
}: Props) {
  const {
    candidateOptions,
    canSave,
    handleSave,
    loadWarning,
    loading,
    saving,
    searchText,
    selectedNames,
    selectedNameSet,
    setSearchText,
    toggleSelected,
  } = useStrategyPoolEditor({
    open,
    group,
    onClose,
    onSaved,
  })

  return (
    <StrategyPoolEditorDialogView
      open={open}
      group={group}
      candidateOptions={candidateOptions}
      canSave={canSave}
      loadWarning={loadWarning}
      loading={loading}
      saving={saving}
      searchText={searchText}
      selectedNames={selectedNames}
      selectedNameSet={selectedNameSet}
      onClose={onClose}
      onSave={handleSave}
      onSearchTextChange={setSearchText}
      onToggleSelected={toggleSelected}
    />
  )
}
