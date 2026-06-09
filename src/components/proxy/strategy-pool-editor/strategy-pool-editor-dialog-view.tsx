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

import type { StrategyPoolGroupRef } from '../strategy-pools/types'
import { StrategyPoolCandidateGrid } from './strategy-pool-candidate-grid'
import type { CandidateOption } from './types'

interface StrategyPoolEditorDialogViewProps {
  open: boolean
  group: StrategyPoolGroupRef | null
  candidateOptions: CandidateOption[]
  canSave: boolean
  loadWarning: string
  loading: boolean
  saving: boolean
  searchText: string
  selectedNames: string[]
  selectedNameSet: ReadonlySet<string>
  onClose: () => void
  onSave: () => void
  onSearchTextChange: (value: string) => void
  onToggleSelected: (name: string, checked?: boolean) => void
}

export function StrategyPoolEditorDialogView({
  open,
  group,
  candidateOptions,
  canSave,
  loadWarning,
  loading,
  saving,
  searchText,
  selectedNames,
  selectedNameSet,
  onClose,
  onSave,
  onSearchTextChange,
  onToggleSelected,
}: StrategyPoolEditorDialogViewProps) {
  const { t } = useTranslation()

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
              这里只认你手动勾选并保存的成员，不再从运行时自动回填。
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Chip size="small" variant="outlined" label={group.displayType} />
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
          onSearch={(_, state) => onSearchTextChange(state.text)}
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
            onToggleSelected={onToggleSelected}
          />
        </div>
      </DialogContent>

      <DialogActions>
        <Button variant="outlined" onClick={onClose} disabled={saving}>
          {t('shared.actions.cancel')}
        </Button>
        <Button onClick={onSave} loading={saving} disabled={!canSave}>
          {t('shared.actions.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
