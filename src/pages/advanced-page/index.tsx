import { useLockFn } from 'ahooks'
import { useState } from 'react'

import { BasePage } from '@/components/base'
import { Alert, Box, Tab, Tabs } from '@/components/tailwind'
import { useConfigLoader, useConfigSaver } from '@/hooks'
import {
  coordinatorGetStatus,
  getAdvancedConfig,
  getRecommendedAdvancedConfig,
  saveAdvancedConfig,
  type AdvancedConfig,
} from '@/services/coordinator'
import { showNotice } from '@/services/notice-service'

import {
  ADVANCED_TABS,
  ADVANCED_TAB_IDS,
  type AdvancedTabId,
} from './constants'
import { AdvancedPageHeader } from './page-header'
import { AdvancedTabContent } from './tab-content'

function getErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message
  }

  return String(error)
}

export default function AdvancedPage() {
  const [activeTab, setActiveTab] = useState<AdvancedTabId>(
    ADVANCED_TAB_IDS.security,
  )
  const [localConfig, setLocalConfig] = useState<AdvancedConfig | null>(null)

  const { data: loadedConfig, loading: configLoading, reload: reloadConfig } =
    useConfigLoader({
      loadFn: getAdvancedConfig,
      onSuccess: (config) => {
        setLocalConfig(config)
      },
    })

  const { data: status, loading: statusLoading, reload: reloadStatus } =
    useConfigLoader({
      loadFn: coordinatorGetStatus,
    })

  const { save, saving } = useConfigSaver({
    saveFn: saveAdvancedConfig,
    onSuccess: () => {
      void reloadConfig()
      void reloadStatus()
    },
    successMessage: '配置已保存并应用。',
  })

  const visibleTabs = ADVANCED_TABS

  const hasUnsavedSecurityPolicies =
    JSON.stringify(localConfig?.security_policies ?? []) !==
    JSON.stringify(loadedConfig?.security_policies ?? [])

  const handleSave = () => {
    if (!localConfig) return
    save(localConfig)
  }

  const handleLoadRecommended = useLockFn(async () => {
    try {
      const recommended = await getRecommendedAdvancedConfig()
      setLocalConfig(recommended)
      showNotice('success', '已加载推荐配置。')
    } catch (error) {
      showNotice('error', '加载推荐配置失败。', getErrorMessage(error))
    }
  })

  if (configLoading || statusLoading || !loadedConfig || !status || !localConfig) {
    return (
      <BasePage title="高级功能">
        <Box className="p-2">加载中...</Box>
      </BasePage>
    )
  }

  return (
    <BasePage
      title="高级功能"
      header={
        <AdvancedPageHeader
          saving={saving}
          onLoadRecommended={() => void handleLoadRecommended()}
          onSave={handleSave}
        />
      }
    >
      {status.securityCompromised ? (
        <Alert severity="error" className="mb-2">
          当前检测到安全状态异常，请尽快检查系统环境、配置来源和运行时状态。
        </Alert>
      ) : null}

      <Box className="border-b border-gray-200 dark:border-gray-700">
        <Tabs
          value={activeTab}
          onChange={(_event, value) => setActiveTab(String(value) as AdvancedTabId)}
          aria-label="高级功能配置"
          variant="scrollable"
          scrollButtons="auto"
        >
          {visibleTabs.map((tab) => (
            <Tab key={tab.id} label={tab.label} value={tab.id} />
          ))}
        </Tabs>
      </Box>

      <AdvancedTabContent
        activeTab={activeTab}
        config={localConfig}
        status={status}
        hasUnsavedSecurityPolicies={hasUnsavedSecurityPolicies}
        onRefreshStatus={reloadStatus}
        onConfigChange={setLocalConfig}
      />
    </BasePage>
  )
}
