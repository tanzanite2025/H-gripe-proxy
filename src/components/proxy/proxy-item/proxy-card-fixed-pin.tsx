import { Pin } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { cn } from '@/utils/cn'

interface ProxyCardFixedPinProps {
  group: IProxyGroupItem
  proxy: IProxyItem
}

export function ProxyCardFixedPin({
  group,
  proxy,
}: ProxyCardFixedPinProps) {
  const { t } = useTranslation()

  if (!group.fixed || group.fixed !== proxy.name) {
    return null
  }

  return (
    <span
      className={cn(
        'absolute -right-1 -top-1 text-xs',
        proxy.name === group.now ? 'the-pin' : 'the-unpin grayscale',
      )}
      title={
        group.type === 'URLTest'
          ? t('proxies.page.labels.delayCheckReset')
          : ''
      }
    >
      <Pin className="h-3.5 w-3.5" />
    </span>
  )
}
