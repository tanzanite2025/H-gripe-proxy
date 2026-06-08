import {
  Check,
  CheckSquare,
  Clipboard,
  FileText,
  Flame,
  MinusSquare,
  RefreshCw,
  Square,
  Trash2,
  X,
} from 'lucide-react'
import { type FC } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseStyledTextField } from '@/components/base'
import { Box, Button, IconButton } from '@/components/tailwind'

export interface ProfileHeaderProps {
  batchMode: boolean
  error: unknown
  isStale: boolean
  selectedCount: number
  isAllSelected: () => boolean
  getSelectionState: () => 'none' | 'all' | 'partial'
  clearAllSelections: () => void
  selectAllProfiles: () => void
  toggleBatchMode: () => void
  onUpdateAll: () => void
  onOpenConfig: () => void
  onReactivate: () => void
  onEmergencyRefresh: () => void
  onDeleteSelectedProfiles: () => void
  onOpenMerge: () => void
  onOpenScript: () => void
  url: string
  setUrl: (url: string) => void
  disabled: boolean
  loading: boolean
  onImport: () => void
  onCopyLink: () => void
  onCreate: () => void
}

export const ProfileHeader: FC<ProfileHeaderProps> = ({
  batchMode,
  error,
  isStale,
  selectedCount,
  isAllSelected,
  getSelectionState,
  clearAllSelections,
  selectAllProfiles,
  toggleBatchMode,
  onUpdateAll,
  onOpenConfig,
  onReactivate,
  onEmergencyRefresh,
  onDeleteSelectedProfiles,
  onOpenMerge,
  onOpenScript,
  url,
  setUrl,
  disabled,
  loading,
  onImport,
  onCopyLink,
  onCreate,
}) => {
  const { t } = useTranslation()

  if (batchMode) {
    return (
      <Box className="flex items-center gap-5 pr-1">
        <IconButton
          size="small"
          color="inherit"
          title={
            isAllSelected()
              ? t('profiles.page.batch.actions.deselectAll')
              : t('profiles.page.batch.actions.selectAll')
          }
          onClick={isAllSelected() ? clearAllSelections : selectAllProfiles}
        >
          {getSelectionState() === 'all' ? (
            <CheckSquare className="h-5 w-5" />
          ) : getSelectionState() === 'partial' ? (
            <MinusSquare className="h-5 w-5" />
          ) : (
            <Square className="h-5 w-5" />
          )}
        </IconButton>

        <IconButton
          size="small"
          color="error"
          title={t('profiles.page.batch.actions.delete')}
          onClick={onDeleteSelectedProfiles}
          disabled={selectedCount === 0}
        >
          <Trash2 className="h-5 w-5" />
        </IconButton>

        <Button size="small" variant="outlined" onClick={toggleBatchMode}>
          {t('profiles.page.batch.actions.done')}
        </Button>

        <Box className="flex-1 text-right text-gray-500 dark:text-gray-400">
          {t('profiles.page.batch.summary.selected')} {selectedCount}{' '}
          {t('profiles.page.batch.summary.items')}
        </Box>
      </Box>
    )
  }

  return (
    <Box className="flex items-center gap-2 pr-1">
      <IconButton
        size="small"
        color="inherit"
        title={t('profiles.page.batch.title')}
        onClick={toggleBatchMode}
      >
        <Check className="h-5 w-5" />
      </IconButton>

      <IconButton
        size="small"
        color="inherit"
        title={t('profiles.page.actions.updateAll')}
        onClick={onUpdateAll}
      >
        <RefreshCw className="h-5 w-5" />
      </IconButton>

      <IconButton
        size="small"
        color="inherit"
        title={t('profiles.page.actions.viewRuntimeConfig')}
        onClick={onOpenConfig}
      >
        <FileText className="h-5 w-5" />
      </IconButton>

      <IconButton
        size="small"
        color="inherit"
        title={t('profiles.page.actions.reactivate')}
        onClick={onReactivate}
      >
        <Flame className="h-5 w-5" />
      </IconButton>

      {(error || isStale) && (
        <IconButton
          size="small"
          color="warning"
          title="数据异常，点击强制刷新"
          onClick={onEmergencyRefresh}
          className="animate-pulse"
        >
          <X className="h-5 w-5" />
        </IconButton>
      )}

      <Box className="min-w-0 flex-1">
        <BaseStyledTextField
          size="small"
          value={url}
          variant="outlined"
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setUrl(event.target.value)
          }
          onKeyDown={(event: React.KeyboardEvent) => {
            if (event.key !== 'Enter' || event.nativeEvent.isComposing) return
            if (!url || disabled || loading) return
            event.preventDefault()
            void onImport()
          }}
          placeholder={t('profiles.page.importForm.placeholder')}
          slotProps={{
            input: {
              sx: { pr: 1 },
              endAdornment: !url ? (
                <IconButton
                  size="small"
                  className="p-0.5"
                  title={t('profiles.page.importForm.actions.paste')}
                  onClick={onCopyLink}
                >
                  <Clipboard className="h-4 w-4" />
                </IconButton>
              ) : (
                <IconButton
                  size="small"
                  className="p-0.5"
                  title={t('shared.actions.clear')}
                  onClick={() => setUrl('')}
                >
                  <X className="h-4 w-4" />
                </IconButton>
              ),
            },
          }}
        />
      </Box>

      <Button
        variant="primary"
        size="small"
        className="shrink-0 rounded-[6px]"
        onClick={onOpenMerge}
      >
        {t('profiles.components.more.global.merge')}
      </Button>

      <Button
        variant="primary"
        size="small"
        className="shrink-0 rounded-[6px]"
        onClick={onOpenScript}
      >
        {t('profiles.components.more.global.script')}
      </Button>

      <Button
        disabled={!url || disabled}
        loading={loading}
        variant="primary"
        size="small"
        className="shrink-0 rounded-[6px]"
        onClick={onImport}
      >
        {t('profiles.page.actions.import')}
      </Button>

      <Button
        variant="primary"
        size="small"
        className="shrink-0 rounded-[6px]"
        onClick={onCreate}
      >
        {t('shared.actions.new')}
      </Button>
    </Box>
  )
}
