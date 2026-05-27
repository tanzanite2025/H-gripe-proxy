import { Code as CodeRounded, Grid as ViewModuleRounded } from 'lucide-react'
import type { ReactNode } from 'react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { IconButton } from '@/components/tailwind/IconButton'
import { TextField } from '@/components/tailwind/TextField'
import { Tooltip } from '@/components/tailwind/Tooltip'

export type BaseSplitChipEditorMode = 'visual' | 'advanced'

interface BaseSplitChipEditorProps {
  value?: string
  onChange: (value: string) => void
  disabled?: boolean
  error?: boolean
  helperText?: ReactNode
  placeholder?: string
  rows?: number
  separator?: string
  splitPattern?: RegExp
  defaultMode?: BaseSplitChipEditorMode
  showModeToggle?: boolean
  ariaLabel?: string
  addLabel?: ReactNode
  emptyLabel?: ReactNode
  modeLabels?: Partial<Record<BaseSplitChipEditorMode, ReactNode>>
  renderHeader?: (modeToggle: ReactNode) => ReactNode
}

const DEFAULT_SPLIT_PATTERN = /[,\n;\r]+/

const splitValue = (value: string, splitPattern: RegExp) =>
  value
    .split(splitPattern)
    .map((item) => item.trim())
    .filter(Boolean)

export const BaseSplitChipEditor = ({
  value = '',
  onChange,
  disabled = false,
  error = false,
  helperText,
  placeholder,
  rows = 4,
  separator = ',',
  splitPattern = DEFAULT_SPLIT_PATTERN,
  defaultMode = 'visual',
  showModeToggle = true,
  ariaLabel,
  addLabel,
  emptyLabel,
  modeLabels,
  renderHeader,
}: BaseSplitChipEditorProps) => {
  const { t } = useTranslation()
  const [mode, setMode] = useState<BaseSplitChipEditorMode>(defaultMode)
  const [draft, setDraft] = useState('')

  const resolvedLabels = useMemo(
    () => ({
      visual: modeLabels?.visual ?? t('shared.editorModes.visualization'),
      advanced: modeLabels?.advanced ?? t('shared.editorModes.advanced'),
      add: addLabel ?? t('shared.actions.new'),
      empty: emptyLabel ?? t('shared.statuses.empty'),
    }),
    [t, modeLabels, addLabel, emptyLabel],
  )

  const values = useMemo(
    () => splitValue(value, splitPattern),
    [value, splitPattern],
  )

  const items = useMemo(() => {
    const counts = new Map<string, number>()
    return values.map((item) => {
      const nextCount = (counts.get(item) ?? 0) + 1
      counts.set(item, nextCount)
      return {
        key: `${item}-${nextCount}`,
        value: item,
      }
    })
  }, [values])

  const handleAddDraft = () => {
    const nextValues = splitValue(draft, splitPattern)
    if (!nextValues.length) {
      return
    }
    const nextValue = [...values, ...nextValues].join(separator)
    onChange(nextValue)
    setDraft('')
  }

  const handleRemoveItem = (index: number) => {
    const nextValue = values.filter((_, itemIndex) => itemIndex !== index)
    onChange(nextValue.join(separator))
  }

  const nextMode = mode === 'visual' ? 'advanced' : 'visual'
  const toggleLabel =
    nextMode === 'visual' ? resolvedLabels.visual : resolvedLabels.advanced
  const ToggleIcon = nextMode === 'visual' ? ViewModuleRounded : CodeRounded
  const resolvedAriaLabel =
    ariaLabel ?? (typeof toggleLabel === 'string' ? toggleLabel : undefined)

  const modeToggle = showModeToggle ? (
    <Tooltip title={toggleLabel}>
      <IconButton
        size="small"
        aria-label={resolvedAriaLabel}
        onClick={() => {
          setMode(nextMode)
          if (nextMode === 'visual') {
            setDraft('')
          }
        }}
      >
        <ToggleIcon className="h-4 w-4" />
      </IconButton>
    </Tooltip>
  ) : null

  return (
    <>
      {renderHeader ? renderHeader(modeToggle) : modeToggle}
      {mode === 'visual' ? (
        <div className="px-0.5 pb-[5px]">
          <div className="flex min-h-8 flex-wrap gap-1">
            {items.length ? (
              items.map((item, index) => (
                <Chip
                  key={item.key}
                  label={item.value}
                  size="small"
                  onDelete={
                    disabled ? undefined : () => handleRemoveItem(index)
                  }
                />
              ))
            ) : (
              <div className="text-sm text-gray-500 dark:text-gray-400">
                {resolvedLabels.empty}
              </div>
            )}
          </div>
          <div className="mt-1 flex items-center gap-1">
            <TextField
              disabled={disabled}
              size="small"
              fullWidth
              value={draft}
              placeholder={placeholder}
              error={error}
              className="[&_.MuiInputBase-root]:min-h-8 [&_.MuiInputBase-input]:px-2 [&_.MuiInputBase-input]:py-1"
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault()
                  handleAddDraft()
                }
              }}
            />
            <Button
              variant="outlined"
              size="small"
              onClick={handleAddDraft}
              disabled={disabled || !draft.trim()}
              className="min-h-8 px-2 py-0.5"
            >
              {resolvedLabels.add}
            </Button>
          </div>
          {helperText && (
            <div className={`mt-1 text-xs ${error ? 'text-red-600 dark:text-red-400' : 'text-gray-500 dark:text-gray-400'}`}>
              {helperText}
            </div>
          )}
        </div>
      ) : (
        <TextField
          error={error}
          disabled={disabled}
          size="small"
          multiline
          rows={rows}
          className="w-full"
          value={value}
          helperText={helperText}
          onChange={(event) => {
            onChange(event.target.value)
          }}
        />
      )}
    </>
  )
}
