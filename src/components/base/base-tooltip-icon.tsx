import { Info } from 'lucide-react'
import type { ComponentProps } from 'react'

import { Tooltip, IconButton } from '@/components/tailwind'

interface Props extends Omit<ComponentProps<typeof IconButton>, 'children'> {
  title?: string
  icon?: React.ComponentType<{ className?: string }>
}

export const TooltipIcon: React.FC<Props> = (props: Props) => {
  const { title = '', icon: Icon = Info, ...restProps } = props

  return (
    <Tooltip title={title}>
      <IconButton size="small" {...restProps}>
        <Icon className="cursor-pointer opacity-75" />
      </IconButton>
    </Tooltip>
  )
}
