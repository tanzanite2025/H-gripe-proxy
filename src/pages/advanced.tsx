/**
 * 高级功能统一配置页面
 */

import { useState, useEffect } from 'react'
import { useLockFn } from 'ahooks'
import { Box, Tabs, Tab, Alert, Button, Stack } from '@mui/material'
import { BasePage } from '@/components/base'
import { Notice } from '@/components/base/base-notice'
import {
  getAdvancedConfig,
  saveAdvancedConfig,
  getRecommendedAdvancedConfig,
  coordinatorGetStatus,
  type AdvancedConfig,
  type CoordinatorStatus,
} from '@/services/coordinator'
import { SecurityConfigPanel } from '@/components/advanced/security-config-panel'
import { MultipathConfigPanel } from '@/components/advanced/multipath-config-panel'
import { XdpConfigPanel } from '@/components/advanced/xdp-config-panel'
import { PerformanceMonitor } from '@/components/advanced/performance-monitor'

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
  const [config, setConfig] = useState<AdvancedConfig | null>(null)
  const [status, setStatus] = useState<CoordinatorStatus | null>(null)
  const [loading, setLoading] = useState(true)

  // 加载配置
  const loadConfig = useLockFn(async () => {
    try {
      setLoading(true)
      const [cfg, st] = await Promise.all([
        getAdvancedConfig(),
        coordinatorGetStatus(),
      ])
      setConfig(cfg)
      setStatus(st)
    } catch (err: any) {
      Notice.error(err.message || err.toString())
    } finally {
      setLoading(false)
    }
  })

  // 保存配置
  const handleSave = useLockFn(async () => {
    if (!config) return

    try {
      await saveAdvancedConfig(config)
      Notice.success('配置已保存并应用')
      await loadConfig()
    } catch (err: any) {
      Notice.error(err.message || err.toString())
    }
  })

  // 加载推荐配置
  const handleLoadRecommended = useLockFn(async () => {
    try {
      const recommended = await getRecommendedAdvancedConfig()
      setConfig(recommended)
      Notice.success('已加载推荐配置')
    } catch (err: any) {
      Notice.error(err.message || err.toString())
    }
  })

  useEffect(() => {
    loadConfig()
  }, [])

  if (loading || !config) {
    return (
      <BasePage title="高级功能" loading={loading}>
        <Box sx={{ p: 2 }}>加载中...</Box>
      </BasePage>
    )
  }

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
          >
            保存配置
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
          config={config.security}
          onChange={(security) => setConfig({ ...config, security })}
        />
      </TabPanel>

      <TabPanel value={tabValue} index={1}>
        <MultipathConfigPanel
          config={config.multipath}
          onChange={(multipath) => setConfig({ ...config, multipath })}
        />
      </TabPanel>

      {window.navigator.platform.toLowerCase().includes('linux') && (
        <TabPanel value={tabValue} index={2}>
          <XdpConfigPanel
            config={config.xdp!}
            onChange={(xdp) => setConfig({ ...config, xdp })}
          />
        </TabPanel>
      )}

      <TabPanel value={tabValue} index={window.navigator.platform.toLowerCase().includes('linux') ? 3 : 2}>
        <PerformanceMonitor status={status} onRefresh={loadConfig} />
      </TabPanel>
    </BasePage>
  )
}
