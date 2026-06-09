import { useTranslation } from 'react-i18next'

import { BaseSearchBox } from '@/components/base'
import {
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind'

import { StrategyPoolCandidateGrid } from './strategy-pool-editor/strategy-pool-candidate-grid'
import { useStrategyPoolEditor } from './strategy-pool-editor/use-strategy-pool-editor'

interface Props {
  open: boolean
  group: IProxyGroupItem | null
  onClose: () => void
  onSaved?: () => Promise<void> | void
}

export function StrategyPoolEditorDialog({
  open,
  group,
  onClose,
  onSaved,
}: Props) {
  const { t } = useTranslation()
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

  if (!group) {
    return null
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      showCloseButton
      maxWidth="md"
      fullWidth
      slotProps={{ paper: { className: 'max-h-full' } }}
    >
      <DialogTitle className="pb-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="text-base font-semibold text-text-primary">
              {group.name}
            </div>
            <div className="mt-1 text-xs text-text-secondary">
              这里不再自动回填运行时成员，只有你手动勾选并保存的节点才算入池。
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Chip size="small" variant="outlined" label={group.type} />
            <Chip
              size="small"
              variant="outlined"
              color="primary"
              label={`已选 ${selectedNames.length}`}
            />
            <Chip
              size="small"
              variant="outlined"
              label={`可选 ${candidateOptions.length}`}
            />
          </div>
        </div>
      </DialogTitle>

      <DialogContent className="space-y-3">
        <BaseSearchBox
          value={searchText}
          onSearch={(_, state) => setSearchText(state.text)}
        />

        {loadWarning ? (
          <div className="rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2 text-xs text-amber-200">
            {loadWarning}
          </div>
        ) : null}

        <div className="rounded-2xl border border-white/8 bg-white/5 p-2">
          <StrategyPoolCandidateGrid
            candidateOptions={candidateOptions}
            loading={loading}
            selectedNameSet={selectedNameSet}
            onToggleSelected={toggleSelected}
          />
        </div>
      </DialogContent>

      <DialogActions>
        <Button variant="outlined" onClick={onClose} disabled={saving}>
          {t('shared.actions.cancel')}
        </Button>
        <Button onClick={handleSave} loading={saving} disabled={!canSave}>
          {t('shared.actions.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
