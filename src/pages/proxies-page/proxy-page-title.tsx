import { AlertTriangle } from 'lucide-react'

import { TooltipIcon } from '@/components/base'
import { Box } from '@/components/tailwind'

interface ProxyPageTitleProps {
  title: string
  warning: string
}

export const ProxyPageTitle = ({
  title,
  warning,
}: ProxyPageTitleProps) => {
  return (
    <Box
      component="span"
      data-tauri-drag-region="true"
      className="inline-flex items-center gap-3"
    >
      {title}
      <TooltipIcon
        title={warning}
        icon={AlertTriangle}
        color="warning"
        className="p-1"
      />
    </Box>
  )
}
