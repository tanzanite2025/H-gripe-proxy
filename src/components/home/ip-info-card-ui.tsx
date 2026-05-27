import { VisibilityOffOutlined, VisibilityOutlined } from '@mui/icons-material'
import { memo } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import { IconButton } from '@/components/tailwind/IconButton'
import { Skeleton } from '@/components/tailwind/Skeleton'

const InfoItem = memo(({ label, value }: { label: string; value?: string }) => (
  <div className="mb-1.5 flex items-start">
    <p className="uds-label text-sm text-text-secondary min-w-[60px] mr-1 flex-shrink-0 text-right">
      {label}:
    </p>
    <p className="text-sm ml-1 overflow-hidden text-ellipsis break-words whitespace-normal flex-grow">
      {value || 'Unknown'}
    </p>
  </div>
))

InfoItem.displayName = 'InfoItem'

// 获取国旗表情
const getCountryFlag = (countryCode: string | undefined) => {
  if (!countryCode) return ''
  const codePoints = countryCode
    .toUpperCase()
    .split('')
    .map((char) => 127397 + char.charCodeAt(0))
  return String.fromCodePoint(...codePoints)
}

interface IPInfoData {
  ip?: string
  country?: string
  country_code?: string
  city?: string
  region?: string
  timezone?: string
  asn?: number
  asn_organization?: string
  organization?: string
  longitude?: number
  latitude?: number
  lastFetchTs?: number
}

interface IPInfoCardUIProps {
  ipInfo?: IPInfoData
  error?: Error | null
  isLoading: boolean
  showIp: boolean
  countdown: { type: 'countdown'; remainingSeconds: number } | { type: 'revalidating' }
  onToggleShowIp: () => void
  onRetry: () => void
}

export const IPInfoCardUI = ({
  ipInfo,
  error,
  isLoading,
  showIp,
  countdown,
  onToggleShowIp,
  onRetry,
}: IPInfoCardUIProps) => {
  const { t } = useTranslation()

  if (isLoading) {
    return (
      <div className="flex flex-col gap-2">
        <Skeleton variant="text" width="60%" height={30} />
        <Skeleton variant="text" width="80%" height={24} />
        <Skeleton variant="text" width="70%" height={24} />
        <Skeleton variant="text" width="50%" height={24} />
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-error">
        <p className="text-base text-error">
          {error instanceof Error
            ? error.message
            : t('home.components.ipInfo.errors.load')}
        </p>
        <Button onClick={onRetry} className="mt-4">
          {t('shared.actions.retry')}
        </Button>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      <div className="flex flex-row flex-1 overflow-hidden">
        {/* 左侧：国家和IP地址 */}
        <div className="w-[40%] overflow-hidden">
          <div className="flex items-center mb-2 overflow-hidden">
            <span className="text-2xl mr-2 inline-block w-7 text-center flex-shrink-0 font-[twemoji_mozilla,sans-serif]">
              {getCountryFlag(ipInfo?.country_code)}
            </span>
            <p className="uds-card-title text-base font-medium overflow-hidden text-ellipsis whitespace-nowrap max-w-full">
              {ipInfo?.country || t('home.components.ipInfo.labels.unknown')}
            </p>
          </div>

          <div className="flex items-center mb-2">
            <p className="uds-label text-sm text-text-secondary flex-shrink-0">
              {t('home.components.ipInfo.labels.ip')}:
            </p>
            <div className="flex items-center ml-2 overflow-hidden max-w-[calc(100%-30px)]">
              <p className="uds-mono text-xs overflow-hidden text-ellipsis break-all">
                {showIp ? ipInfo?.ip : '••••••••••'}
              </p>
              <IconButton size="small" onClick={onToggleShowIp}>
                {showIp ? (
                  <VisibilityOffOutlined fontSize="small" />
                ) : (
                  <VisibilityOutlined fontSize="small" />
                )}
              </IconButton>
            </div>
          </div>

          <InfoItem
            label={t('home.components.ipInfo.labels.asn')}
            value={ipInfo?.asn ? `AS${ipInfo.asn}` : 'N/A'}
          />
        </div>

        {/* 右侧：组织、ISP和位置信息 */}
        <div className="w-[60%] overflow-auto">
          <InfoItem
            label={t('home.components.ipInfo.labels.isp')}
            value={ipInfo?.organization}
          />
          <InfoItem
            label={t('home.components.ipInfo.labels.org')}
            value={ipInfo?.asn_organization}
          />
          <InfoItem
            label={t('home.components.ipInfo.labels.location')}
            value={[ipInfo?.city, ipInfo?.region].filter(Boolean).join(', ')}
          />
          <InfoItem
            label={t('home.components.ipInfo.labels.timezone')}
            value={ipInfo?.timezone}
          />
        </div>
      </div>

      <div className="mt-auto pt-1 border-t border-divider flex justify-between items-center opacity-70 text-[0.7rem]">
        <p className="text-xs">
          {t('home.components.ipInfo.labels.autoRefresh')}
          {countdown.type === 'countdown'
            ? `: ${countdown.remainingSeconds}s`
            : '...'}
        </p>
        <p className="uds-mono text-xs text-ellipsis overflow-hidden whitespace-nowrap">
          {`${ipInfo?.country_code ?? 'N/A'}, ${ipInfo?.longitude?.toFixed(2) ?? 'N/A'}, ${ipInfo?.latitude?.toFixed(2) ?? 'N/A'}`}
        </p>
      </div>
    </div>
  )
}
