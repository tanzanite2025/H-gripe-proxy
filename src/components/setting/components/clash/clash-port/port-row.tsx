import { Shuffle } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Switch } from '@/components/base'
import { IconButton, ListItem, ListItemText, TextField } from '@/components/tailwind'

interface ClashPortRowProps {
  label: string
  port: number
  enabled: boolean
  enableToggle: boolean
  randomTitle: string
  onPortChange: (port: number) => void
  onRandomPort: () => void
  onToggleEnabled?: (enabled: boolean) => void
}

export function ClashPortRow({
  label,
  port,
  enabled,
  enableToggle,
  randomTitle,
  onPortChange,
  onRandomPort,
  onToggleEnabled,
}: ClashPortRowProps) {
  return (
    <ListItem className="min-h-[36px] px-0 py-1">
      <ListItemText
        primary={label}
        slotProps={{ primary: { className: 'text-xs' } }}
      />
      <div className="flex items-center">
        <TextField
          size="small"
          className="mr-2 w-[80px] text-xs"
          value={port}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onPortChange(+event.target.value.replace(/\D+/, '').slice(0, 5))
          }
          disabled={!enabled}
        />
        <IconButton
          size="small"
          onClick={onRandomPort}
          title={randomTitle}
          disabled={!enabled}
          className="mr-2"
        >
          <Shuffle className="h-4 w-4" />
        </IconButton>
        <Switch
          size="small"
          checked={enabled}
          disabled={!enableToggle}
          onCheckedChange={onToggleEnabled}
          className="ml-2"
        />
      </div>
    </ListItem>
  )
}
