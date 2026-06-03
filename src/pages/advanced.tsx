/**
 * 高级功能统一配置页面
 */

import { useLockFn } from 'ahooks'
import { useState } from 'react'

import { BlackholeBreakerPanel } from '@/components/advanced/blackhole-breaker-panel'
import { EgressIdentityPanel } from '@/components/advanced/egress-identity-panel'
import { EgressMonitorPanel } from '@/components/advanced/egress-monitor-panel'
import { IpReputationPanel } from '@/components/advanced/ip-reputation-panel'
import { MultipathConfigPanel } from '@/components/advanced/multipath-config-panel'
import { PerformanceMonitor } from '@/components/advanced/performance-monitor'
import { ResidentialPoolPanel } from '@/components/advanced/residential-pool-panel'
import { SecurityConfigPanel } from '@/components/advanced/security-config-panel'
import { SecurityPolicyPanel } from '@/components/advanced/security-policy-panel'
import { TimezoneSpoofPanel } from '@/components/advanced/timezone-spoof-panel'
import { XdpConfigPanel } from '@/components/advanced/xdp-config-panel'
import { BasePage } from '@/components/base'
import { IngressCountermeasurePanel } from '@/components/security/ingress-countermeasure-panel'
import { LocalStealthPanel } from '@/components/security/local-stealth-panel'
import { SessionAffinityBindings as SessionAffinityBindingsPanel } from '@/components/security/session-affinity-bindings'
import { SessionAffinityConfig as SessionAffinityConfigPanel } from '@/components/security/session-affinity-config'
import { Box, Tabs, Tab, Alert, Button, Stack } from '@/components/tailwind'
import { useConfigLoader, useConfigSaver } from '@/hooks'
import {
  getAdvancedConfig,
  saveAdvancedConfig,
  getRecommendedAdvancedConfig,
  coordinatorGetStatus,
  type AdvancedConfig,
} from '@/services/coordinator'
import { showNotice } from '@/services/notice-service'

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
      {value === index && <Box className="py-2">{children}</Box>}
    </div>
  )
}

export default function AdvancedPage() {
  const isLinux = window.navigator.platform.toLowerCase().includes('linux')
  const [tabValue, setTabValue] = useState(0)
  const [localConfig, setLocalConfig] = useState<AdvancedConfig | null>(null)

  // 使用通用 Hook 分别加载配置和运行态状态，避免刷新状态时覆盖未保存草稿
  const { data: loadedConfig, loading: configLoading, reload: reloadConfig } = useConfigLoader({
    loadFn: getAdvancedConfig,
    onSuccess: (config) => {
      setLocalConfig(config)
    },
  })
  const { data: status, loading: statusLoading, reload: reloadStatus } = useConfigLoader({
    loadFn: coordinatorGetStatus,
  })

  // 使用通用 Hook 保存配置
  const { save, saving } = useConfigSaver({
    saveFn: saveAdvancedConfig,
    onSuccess: () => {
      void reloadConfig()
      void reloadStatus()
    },
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

  const securityTabIndex = 0
  const securityPolicyTabIndex = 1
  const localStealthTabIndex = 2
  const egressIdentityTabIndex = 3
  const sessionAffinityTabIndex = 4
  const egressMonitorTabIndex = 5
  const residentialPoolTabIndex = 6
  const ipReputationTabIndex = 7
  const blackholeBreakerTabIndex = 8
  const timezoneSpoofTabIndex = 9
  const multipathTabIndex = 10
  const xdpTabIndex = 11
  const performanceTabIndex = isLinux ? 12 : 11


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
        <Stack direction="row" spacing={1}>
          <Button
            variant="outlined"
            size="small"
            onClick={handleLoadRecommended}
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
      }
    >
      {status?.securityCompromised && (
        <Alert severity="error" className="mb-2">
          ⚠️ 安全状态已被破坏！请立即检查系统安全。
        </Alert>
      )}

      <Box className="border-b border-gray-200 dark:border-gray-700">
        <Tabs
          value={tabValue}
          onChange={(_, v) => setTabValue(Number(v))}
          aria-label="高级功能配置"
          variant="scrollable"
          scrollButtons="auto"
        >
          <Tab label="安全防御" value={securityTabIndex} />
          <Tab label="安全策略" value={securityPolicyTabIndex} />
          <Tab label="本地隐蔽" value={localStealthTabIndex} />
          <Tab label="出口身份" value={egressIdentityTabIndex} />
          <Tab label="会话绑定" value={sessionAffinityTabIndex} />
          <Tab label="出口监控" value={egressMonitorTabIndex} />
          <Tab label="住宅代理池" value={residentialPoolTabIndex} />
          <Tab label="IP 信誉" value={ipReputationTabIndex} />
          <Tab label="黑洞熔断" value={blackholeBreakerTabIndex} />
          <Tab label="时区伪装" value={timezoneSpoofTabIndex} />
          <Tab label="多路径路由" value={multipathTabIndex} />
          {isLinux && (
            <Tab label="XDP 代理" value={xdpTabIndex} />
          )}
          <Tab label="性能监控" value={performanceTabIndex} />
        </Tabs>
      </Box>

      <TabPanel value={tabValue} index={securityTabIndex}>
        <div className="space-y-4">
          <SecurityConfigPanel
            config={localConfig.security}
            onChange={(security) => setLocalConfig({ ...localConfig, security })}
          />
          <IngressCountermeasurePanel
            config={localConfig.ingress_countermeasure}
            onChange={(ingress_countermeasure) =>
              setLocalConfig({ ...localConfig, ingress_countermeasure })
            }
          />
        </div>
      </TabPanel>

      <TabPanel value={tabValue} index={securityPolicyTabIndex}>
        <SecurityPolicyPanel
          policies={localConfig.security_policies ?? []}
          onChange={(security_policies) =>
            setLocalConfig({ ...localConfig, security_policies })
          }
        />
      </TabPanel>

      <TabPanel value={tabValue} index={localStealthTabIndex}>
        <LocalStealthPanel />
      </TabPanel>

      <TabPanel value={tabValue} index={egressIdentityTabIndex}>
        <EgressIdentityPanel
          config={localConfig.egress_identity}
          status={status}
          onRefreshStatus={reloadStatus}
          residentialPool={localConfig.residential_pool}
          onChange={(egress_identity) =>
            setLocalConfig({ ...localConfig, egress_identity })
          }
        />
      </TabPanel>

      <TabPanel value={tabValue} index={sessionAffinityTabIndex}>
        <div className="space-y-4">
          <SessionAffinityConfigPanel
            config={localConfig.session_affinity}
            onChange={(session_affinity) =>
              setLocalConfig({ ...localConfig, session_affinity })
            }
          />
          <SessionAffinityBindingsPanel status={status} onRefreshStatus={reloadStatus} />
        </div>
      </TabPanel>

      <TabPanel value={tabValue} index={egressMonitorTabIndex}>
        <EgressMonitorPanel
          config={localConfig.egress_monitor}
          onChange={(egress_monitor) =>
            setLocalConfig({ ...localConfig, egress_monitor })
          }
        />
      </TabPanel>

      <TabPanel value={tabValue} index={residentialPoolTabIndex}>
        <ResidentialPoolPanel
          config={localConfig.residential_pool}
          onChange={(residential_pool) =>
            setLocalConfig({ ...localConfig, residential_pool })
          }
        />
      </TabPanel>

      <TabPanel value={tabValue} index={ipReputationTabIndex}>
        <IpReputationPanel
          config={localConfig.ip_reputation}
          onChange={(ip_reputation) =>
            setLocalConfig({ ...localConfig, ip_reputation })
          }
        />
      </TabPanel>

      <TabPanel value={tabValue} index={blackholeBreakerTabIndex}>
        <BlackholeBreakerPanel
          config={localConfig.blackhole_breaker}
          onChange={(blackhole_breaker) =>
            setLocalConfig({ ...localConfig, blackhole_breaker })
          }
        />
      </TabPanel>

      <TabPanel value={tabValue} index={timezoneSpoofTabIndex}>
        <TimezoneSpoofPanel
          config={localConfig.timezone_spoof}
          onChange={(timezone_spoof) =>
            setLocalConfig({ ...localConfig, timezone_spoof })
          }
        />
      </TabPanel>

      <TabPanel value={tabValue} index={multipathTabIndex}>
        <MultipathConfigPanel
          config={localConfig.multipath}
          onChange={(multipath) => setLocalConfig({ ...localConfig, multipath })}
        />
      </TabPanel>

      {isLinux && (
        <TabPanel value={tabValue} index={xdpTabIndex}>
          <XdpConfigPanel
            config={localConfig.xdp!}
            onChange={(xdp) => setLocalConfig({ ...localConfig, xdp })}
          />
        </TabPanel>
      )}

      <TabPanel value={tabValue} index={performanceTabIndex}>
        <PerformanceMonitor status={status} onRefresh={reloadStatus} />
      </TabPanel>
    </BasePage>
  )
}
