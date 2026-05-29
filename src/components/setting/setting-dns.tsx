import { useQuery } from '@tanstack/react-query'
import { useLockFn } from 'ahooks'
import { useState } from 'react'

import { DnsAdvancedPanel } from '@/components/advanced/dns-advanced-panel'
import { Box, Button, Stack } from '@/components/tailwind'
import { useConfigLoader, useConfigSaver } from '@/hooks'
import { useVerge } from '@/hooks/system'
import { applyDnsConfig, getDnsRuntimeStatus } from '@/services/cmds'
import {
  getAdvancedConfig,
  getRecommendedAdvancedConfig,
  saveAdvancedConfig,
  type AdvancedConfig,
} from '@/services/coordinator'
import { showNotice } from '@/services/notice-service'

export default function SettingDns() {
  const [localConfig, setLocalConfig] = useState<AdvancedConfig | null>(null)
  const [persistedConfig, setPersistedConfig] = useState<AdvancedConfig | null>(null)
  const [dnsRuntimePending, setDnsRuntimePending] = useState(false)
  const [runtimeStatusRefreshKey, setRuntimeStatusRefreshKey] = useState(0)
  const { verge, setDnsRuntimeEnabled } = useVerge()

  const {
    data: dnsRuntimeStatus,
    isPending: dnsRuntimeStatusPending,
    refetch: refetchDnsRuntimeStatus,
  } = useQuery({
    queryKey: ['getDnsRuntimeStatus', runtimeStatusRefreshKey],
    queryFn: getDnsRuntimeStatus,
    enabled: !!verge,
  })

  const { loading, reload } = useConfigLoader<AdvancedConfig>({
    loadFn: getAdvancedConfig,
    onSuccess: (config) => {
      setLocalConfig(config)
      setPersistedConfig(config)
    },
  })

  const { save, saving } = useConfigSaver<AdvancedConfig>({
    saveFn: saveAdvancedConfig,
    showSuccessNotice: false,
  })

  const handleLoadRecommended = useLockFn(async () => {
    try {
      const recommended = await getRecommendedAdvancedConfig()
      setLocalConfig((current) =>
        current ? { ...current, dns: recommended.dns } : current,
      )
      showNotice.success('已加载推荐 DNS 配置')
    } catch (err: any) {
      showNotice.error(err)
    }
  })

  const handleSave = useLockFn(async () => {
    if (localConfig) {
      const saved = await save(localConfig)

      if (!saved) {
        return
      }

      if (verge?.enable_dns_settings) {
        await applyDnsConfig(true)
        showNotice.success('DNS 配置已保存并重新应用到运行时')
      } else {
        showNotice.success('DNS 配置已保存')
      }

      setRuntimeStatusRefreshKey((current) => current + 1)
      await reload()
    }
  })

  const handleDnsRuntimeToggle = useLockFn(async (enable: boolean) => {
    setDnsRuntimePending(true)

    try {
      await setDnsRuntimeEnabled(enable)

      setRuntimeStatusRefreshKey((current) => current + 1)

      showNotice.success(
        enable ? 'DNS 配置已应用到运行时' : 'DNS 配置运行时应用已关闭',
      )
    } catch (err: any) {
      showNotice.error(err)
    } finally {
      setDnsRuntimePending(false)
    }
  })

  if (loading || !localConfig || !persistedConfig || !verge) {
    return (
      <div>
        <div className="uds-label mb-4">DNS</div>
        <Box className="p-2">加载中...</Box>
      </div>
    )
  }

  const hasUnsavedDnsChanges =
    JSON.stringify(localConfig.dns) !== JSON.stringify(persistedConfig.dns)

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div className="uds-label">DNS</div>
        <Stack direction="row" spacing={1}>
          <Button
            variant="outlined"
            size="small"
            onClick={handleLoadRecommended}
            disabled={saving}
          >
            加载推荐配置
          </Button>
          <Button
            variant="primary"
            size="small"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? '保存中...' : '保存配置'}
          </Button>
        </Stack>
      </div>

      <DnsAdvancedPanel
        config={localConfig.dns}
        savedConfig={persistedConfig.dns}
        hasUnsavedChanges={hasUnsavedDnsChanges}
        runtimeStatus={dnsRuntimeStatus}
        runtimeStatusPending={dnsRuntimeStatusPending}
        onRuntimeStatusRefresh={() => void refetchDnsRuntimeStatus()}
        onChange={(dns) =>
          setLocalConfig((current) => (current ? { ...current, dns } : current))
        }
        runtimeEnabled={verge.enable_dns_settings ?? false}
        runtimePending={dnsRuntimePending}
        onRuntimeToggle={handleDnsRuntimeToggle}
      />
    </div>
  )
}
