import { Check, X, Trash2, Edit, ExternalLink } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Stack, TextField, IconButton, Divider, Box } from '@/components/tailwind'

interface Props {
  value?: string
  onlyEdit?: boolean
  onChange: (value?: string) => void
  onOpenUrl?: (value?: string) => void
  onDelete?: () => void
  onCancel?: () => void
}

export const WebUIItem = (props: Props) => {
  const {
    value,
    onlyEdit = false,
    onChange,
    onDelete,
    onOpenUrl,
    onCancel,
  } = props

  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState(value)
  const { t } = useTranslation()

  const highlightedParts = useMemo(() => {
    const placeholderRegex = /(%host|%port|%secret)/g
    if (!value) {
      return ['NULL']
    }
    return value.split(placeholderRegex).filter((part) => part !== '')
  }, [value])

  if (editing || onlyEdit) {
    return (
      <>
        <Stack direction="row" spacing={0.75} className="mt-4 mb-4 items-center">
          <TextField
            autoComplete="new-password"
            fullWidth
            size="small"
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            placeholder={t(
              'settings.modals.webUI.messages.supportedPlaceholders',
            )}
          />
          <IconButton
            size="small"
            title={t('shared.actions.save')}
            onClick={() => {
              onChange(editValue)
              setEditing(false)
            }}
          >
            <Check size={16} />
          </IconButton>
          <IconButton
            size="small"
            title={t('shared.actions.cancel')}
            onClick={() => {
              onCancel?.()
              setEditing(false)
            }}
          >
            <X size={16} />
          </IconButton>
        </Stack>
        <Divider />
      </>
    )
  }

  const renderedParts = highlightedParts.map((part, index) => {
    const isPlaceholder =
      part === '%host' || part === '%port' || part === '%secret'
    const repeatIndex = highlightedParts
      .slice(0, index)
      .filter((prev) => prev === part).length
    const key = `${part || 'empty'}-${repeatIndex}`

    return (
      <span key={key} className={isPlaceholder ? 'text-primary' : undefined}>
        {part}
      </span>
    )
  })

  return (
    <>
      <Stack direction="row" spacing={0.75} className="items-center mt-4 mb-4">
        <Box
          title={value}
          className={`w-full overflow-hidden text-ellipsis ${
            value ? 'text-text-primary' : 'text-text-secondary'
          }`}
        >
          {renderedParts}
        </Box>
        <IconButton
          size="small"
          title={t('settings.modals.webUI.actions.openUrl')}
          onClick={() => onOpenUrl?.(value)}
        >
          <ExternalLink size={16} />
        </IconButton>
        <IconButton
          size="small"
          title={t('shared.actions.edit')}
          onClick={() => {
            setEditing(true)
            setEditValue(value)
          }}
        >
          <Edit size={16} />
        </IconButton>
        <IconButton
          size="small"
          title={t('shared.actions.delete')}
          onClick={onDelete}
        >
          <Trash2 size={16} />
        </IconButton>
      </Stack>
      <Divider />
    </>
  )
}
