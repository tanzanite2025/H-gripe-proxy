import dayjs from 'dayjs'
import { Activity, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { IconButton } from '@/components/tailwind/IconButton'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import { ListItem, ListItemText } from '@/components/tailwind/List'
import type { RuntimeProviderHealthRecord } from '@/services/proxy-runtime'
import type { ProxyProvider } from '@/types/mihomo'
import { cn } from '@/utils/cn'
import parseTraffic from '@/utils/format'

import { getProviderProgress, parseExpire } from './utils'

interface ProviderListItemProps {
  name: string
  provider: ProxyProvider
  isUpdating: boolean
  isChecking: boolean
  health?: RuntimeProviderHealthRecord
  onUpdate: (name: string) => void
  onCheck: (name: string) => void
}

export const ProviderListItem = ({
  name,
  provider,
  isUpdating,
  isChecking,
  health,
  onUpdate,
  onCheck,
}: ProviderListItemProps) => {
  const { t } = useTranslation()
  const time = dayjs(provider.updatedAt)
  const { hasSubInfo, upload, download, total, expire, progress } =
    getProviderProgress(provider)

  const healthStatus = health
    ? health.success
      ? 'healthy'
      : 'unhealthy'
    : 'unknown'
  const healthLabel = t(`proxies.page.provider.health.${healthStatus}`)
  const healthTitle =
    health && !health.success && health.error
      ? `${healthLabel}: ${health.error}`
      : health
        ? `${healthLabel} · ${t('proxies.page.provider.health.lastChecked')}: ${dayjs(health.updatedAt).fromNow()}`
        : healthLabel

  return (
    <ListItem
      className={cn(
        'uds-card-container mb-2 overflow-hidden rounded-lg bg-white p-0 transition-all duration-200',
        'hover:bg-primary/10 dark:bg-[#24252f] dark:hover:bg-primary/20',
      )}
    >
      <ListItemText
        className="px-4 py-2"
        primary={
          <div className="flex items-center justify-between">
            <div className="uds-card-title flex items-center overflow-hidden">
              <span className="mr-2 truncate" title={name}>
                {name}
              </span>
              <span className="mr-1 inline-block rounded border border-secondary/50 px-0.5 text-[10px] leading-tight text-secondary/80">
                {provider.proxies.length}
              </span>
              <span className="inline-block rounded border border-secondary/50 px-0.5 text-[10px] leading-tight text-secondary/80">
                {provider.vehicleType}
              </span>
              <span
                title={healthTitle}
                className={cn(
                  'ml-1 inline-flex items-center gap-1 rounded px-1 text-[10px] leading-tight',
                  healthStatus === 'healthy' &&
                    'bg-green-500/10 text-green-600 dark:text-green-400',
                  healthStatus === 'unhealthy' &&
                    'bg-red-500/10 text-red-600 dark:text-red-400',
                  healthStatus === 'unknown' &&
                    'bg-secondary/10 text-text-secondary',
                )}
              >
                <span
                  className={cn(
                    'inline-block h-1.5 w-1.5 rounded-full',
                    healthStatus === 'healthy' && 'bg-green-500',
                    healthStatus === 'unhealthy' && 'bg-red-500',
                    healthStatus === 'unknown' && 'bg-secondary/50',
                  )}
                />
                {healthLabel}
              </span>
            </div>

            <div className="uds-desc ml-2 whitespace-nowrap text-sm text-text-secondary">
              <small>{t('shared.labels.updateAt')}: </small>
              {time.fromNow()}
            </div>
          </div>
        }
        secondary={
          hasSubInfo ? (
            <>
              <div className="mb-2 flex items-center justify-between">
                <span title={t('shared.labels.usedTotal') as string}>
                  {parseTraffic(upload + download)} / {parseTraffic(total)}
                </span>
                <span title={t('shared.labels.expireTime') as string}>
                  {parseExpire(expire)}
                </span>
              </div>

              <LinearProgress
                variant="determinate"
                value={progress}
                className={cn(
                  'h-1.5 rounded-full',
                  total > 0 ? 'opacity-100' : 'opacity-0',
                )}
              />
            </>
          ) : null
        }
      />

      <div className="w-px self-stretch bg-divider" />

      <div className="flex w-10 items-center justify-center">
        <IconButton
          size="small"
          color="primary"
          onClick={() => onCheck(name)}
          disabled={isChecking}
          className={cn(isChecking && 'animate-pulse')}
          title={t('proxies.page.provider.actions.healthCheck')}
          aria-label={t('proxies.page.provider.actions.healthCheck')}
        >
          <Activity className="h-4 w-4" />
        </IconButton>
      </div>

      <div className="w-px self-stretch bg-divider" />

      <div className="flex w-10 items-center justify-center">
        <IconButton
          size="small"
          color="primary"
          onClick={() => onUpdate(name)}
          disabled={isUpdating}
          className={cn(isUpdating && 'animate-spin')}
          title={t('proxies.page.provider.actions.update')}
          aria-label={t('proxies.page.provider.actions.update')}
        >
          <RefreshCw className="h-4 w-4" />
        </IconButton>
      </div>
    </ListItem>
  )
}
