/**
 * 高级功能统一配置页面
 */

import { useState } from 'react'
import { useLockFn } from 'ahooks'
import { Box, Tabs, Tab, Alert, Button, Stack } from '@mui/material'
import { BasePage } from '@/components/base'
import { showNotice } from '@/services/notice-service'
import {
  getAdvancedConfig,
  saveAdvancedConfig,
  getRecommendedAdvancedConfig,
  coordinatorGetStatus,
  type AdvancedConfig,
} from '@/services/coordinator'
import { SecurityConfigPanel } from '@/components/advanced/security-config-panel'
import { MultipathConfigPanel } from '@/components/advanced/multipath-config-panel'
import { XdpConfigPanel } from '@/components/advanced/xdp-config-panel'
import { PerformanceMonitor } from '@/components/advanced/performance-monitor'
import { useMultiConfigLoader, useConfigSaver } from '@/hooks'

interface TabPanelProps {
  children?: React.ReactNode
  index: number
  value: number
}

function TabPanel(props: TabPanelProps) {
  const { children, value, index, ...other } = props

  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`advanced-tabpanel-${index}`}
      aria-labelledby={`advanced-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ py: 2 }}>{children}</Box>}
    </div>
  )
}

export default function AdvancedPage() {
  const [tabValue, setTabValue] = useState(0)
  const [localConfig, setLocalConfig] = useState<AdvancedConfig | null>(null)

  // 使用通用 Hook 加载配置和状态
  const { data, loading, reload } = useMultiConfigLoader({
    loaders: {
      config: getAdvancedConfig,
      status: coordinatorGetStatus,
    },
    onSuccess: (result) => {
      setLocalConfig(result.config)
    },
  })

  // 使用通用 Hook 保存配置
  const { save, saving } = useConfigSaver({
    saveFn: saveAdvancedConfig,
    onSuccess: reload,
    successMessage: '配置已保存并应用',
  })

  // 保存配置
  const handleSave = () => {
    if (localConfig) {
      save(localConfig)
    }
  }

  // 加载推荐配置
  const handleLoadRecommended = useLockFn(async () => {
    try {
      const recommended = await getRecommendedAdvancedConfig()
      setLocalConfig(recommended)
      showNotice('success', '已加载推荐配置')
    } catch (err: any) {
      showNotice('error', err.message || err.toString())
    }
  })

  if (loading || !data || !localConfig) {
    return (
      <BasePage title="高级功能">
        <Box sx={{ p: 2 }}>加载中...</Box>
      </BasePage>
    )
  }

  const { config, status } = data

  return (
    <BasePage
      title="高级功能"
      header={
        <Stack direction="row" spacing={1}>
          <Button
            variant="outlined"
            size="small"
            onClick={handleLoadRecommended}
          >
            加载推荐配置
          </Button>
          <Button
            variant="contained"
            size="small"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? '保存中...' : '保存配置'}
          </Button>
        </Stack>
      }
    >
      {status?.security_compromised && (
        <Alert severity="error" sx={{ mb: 2 }}>
          ⚠️ 安全状态已被破坏！请立即检查系统安全。
        </Alert>
      )}

      <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
        <Tabs
          value={tabValue}
          onChange={(_, v) => setTabValue(v)}
          aria-label="高级功能配置"
        >
          <Tab label="安全防御" />
          <Tab label="多路径路由" />
          {window.navigator.platform.toLowerCase().includes('linux') && (
            <Tab label="XDP 代理" />
          )}
          <Tab label="性能监控" />
        </Tabs>
      </Box>

      <TabPanel value={tabValue} index={0}>
        <SecurityConfigPanel
          config={localConfig.security}
          onChange={(security) => setLocalConfig({ ...localConfig, security })}
        />
      </TabPanel>

      <TabPanel value={tabValue} index={1}>
        <MultipathConfigPanel
          config={localConfig.multipath}
          onChange={(multipath) => setLocalConfig({ ...localConfig, multipath })}
        />
      </TabPanel>

      {window.navigator.platform.toLowerCase().includes('linux') && (
        <TabPanel value={tabValue} index={2}>
          <XdpConfigPanel
            config={localConfig.xdp!}
            onChange={(xdp) => setLocalConfig({ ...localConfig, xdp })}
          />
        </TabPanel>
      )}

      <TabPanel value={tabValue} index={window.navigator.platform.toLowerCase().includes('linux') ? 3 : 2}>
        <PerformanceMonitor status={status} onRefresh={reload} />
      </TabPanel>
    </BasePage>
  )
}
