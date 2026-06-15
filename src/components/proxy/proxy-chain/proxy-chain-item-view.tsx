import { GripVertical, Trash2 } from 'lucide-react'
import type { CSSProperties, HTMLAttributes } from 'react'

import { Chip } from '@/components/tailwind/Chip'
import { IconButton } from '@/components/tailwind/IconButton'

import type { ProxyChainItem } from '../proxy-chain-types'

interface ProxyChainItemViewProps {
  proxy: ProxyChainItem
  index: number
  roleLabel?: string
  roleChipClassName: string
  borderClassName: string
  delayLabel?: string
  delayColor?: 'success' | 'warning' | 'error'
  isDragging: boolean
  style: CSSProperties
  dragHandleProps: HTMLAttributes<HTMLDivElement>
  onRemove: (id: string) => void
}

export const ProxyChainItemView = ({
  proxy,
  index,
  roleLabel,
  roleChipClassName,
  borderClassName,
  delayLabel,
  delayColor,
  isDragging,
  style,
  dragHandleProps,
  onRemove,
}: ProxyChainItemViewProps) => {
  return (
    <div
      style={style}
      className={`mb-0 flex items-center rounded p-2 transition-all duration-200 ${
        isDragging ? 'bg-background shadow-lg' : 'bg-card shadow'
      } ${borderClassName}`}
    >
      <div
        {...dragHandleProps}
        className="mr-2 flex cursor-grab items-center text-text-secondary active:cursor-grabbing"
      >
        <GripVertical className="h-5 w-5" />
      </div>

      {roleLabel ? (
        <Chip
          label={roleLabel}
          size="small"
          className={roleChipClassName}
        />
      ) : (
        <Chip
          label={`${index + 1}`}
          size="small"
          color="primary"
          className="mr-2 min-w-[32px]"
        />
      )}

      <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-sm font-medium">
        {proxy.name}
      </span>

      {proxy.type && (
        <Chip
          label={proxy.type}
          size="small"
          variant="outlined"
          className="mr-2"
        />
      )}

      {delayLabel && delayColor && (
        <Chip
          label={delayLabel}
          size="small"
          color={delayColor}
          className="mr-2 min-w-[50px] text-xs"
        />
      )}

      <IconButton
        size="small"
        onClick={() => onRemove(proxy.id)}
        className="text-red-500 hover:bg-red-500/10"
      >
        <Trash2 className="h-4 w-4" />
      </IconButton>
    </div>
  )
}
