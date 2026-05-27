import { Cpu } from 'lucide-react'
import { Divider, Stack } from '@/components/tailwind'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { useClash } from '@/hooks/data'
import {
  useClashConfigData,
  useRulesData,
  useSystemData,
  useUptimeData,
} from '@/providers/app-data-context'

import { EnhancedCard } from './enhanced-card'

// 将毫秒转换为时:分:秒格式的函数
const formatUptime = (uptimeMs: number) => {
  const hours = Math.floor(uptimeMs / 3600000)
  const minutes = Math.floor((uptimeMs % 3600000) / 60000)
  const seconds = Math.floor((uptimeMs % 60000) / 1000)
  return `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
}

export const ClashInfoCard = () => {
  const { t } = useTranslation()
  const { version: clashVersion } = useClash()
  const { clashConfig } = useClashConfigData()
  const { rules } = useRulesData()
  const { uptime } = useUptimeData()
  const { systemProxyAddress } = useSystemData()

  // 使用useMemo缓存格式化后的uptime，避免频繁计算
  const formattedUptime = useMemo(() => formatUptime(uptime), [uptime])

  // 使用备忘录组件内容，减少重新渲染
  const cardContent = useMemo(() => {
    if (!clashConfig) return null

    return (
      <Stack spacing={1.5}>
        <div className="flex justify-between">
          <span className="text-sm text-gray-600 dark:text-gray-400">
            {t('home.components.clashInfo.fields.coreVersion')}
          </span>
          <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {clashVersion || '-'}
          </span>
        </div>
        <Divider />
        <div className="flex justify-between">
          <span className="text-sm text-gray-600 dark:text-gray-400">
            {t('home.components.clashInfo.fields.systemProxyAddress')}
          </span>
          <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {systemProxyAddress}
          </span>
        </div>
        <Divider />
        <div className="flex justify-between">
          <span className="text-sm text-gray-600 dark:text-gray-400">
            {t('home.components.clashInfo.fields.mixedPort')}
          </span>
          <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {clashConfig.mixedPort || '-'}
          </span>
        </div>
        <Divider />
        <div className="flex justify-between">
          <span className="text-sm text-gray-600 dark:text-gray-400">
            {t('home.components.clashInfo.fields.uptime')}
          </span>
          <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {formattedUptime}
          </span>
        </div>
        <Divider />
        <div className="flex justify-between">
          <span className="text-sm text-gray-600 dark:text-gray-400">
            {t('home.components.clashInfo.fields.rulesCount')}
          </span>
          <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {rules.length}
          </span>
        </div>
      </Stack>
    )
  }, [
    clashConfig,
    clashVersion,
    t,
    formattedUptime,
    rules.length,
    systemProxyAddress,
  ])

  return (
    <EnhancedCard
      title={t('home.components.clashInfo.title')}
      icon={<Cpu />}
      iconColor="warning"
      action={null}
    >
      {cardContent}
    </EnhancedCard>
  )
}
