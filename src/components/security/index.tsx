/**
 * 安全配置主组件
 */

import { useState, type SyntheticEvent } from 'react'

import { Tabs, Tab } from '@/components/tailwind'

import AntiProbeConfigComponent from './anti-probe-config'
import SecurityMonitor from './security-monitor'
import SessionAffinityConfigComponent from './session-affinity-config-wrapper'
import TlsFingerprintSelector from './tls-fingerprint-selector'

interface TabPanelProps {
  children?: React.ReactNode
  index: number
  mounted: boolean
  value: number
}

function TabPanel(props: TabPanelProps) {
  const { children, value, index, mounted, ...other } = props

  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`security-tabpanel-${index}`}
      aria-labelledby={`security-tab-${index}`}
      {...other}
    >
      {mounted && <div>{children}</div>}
    </div>
  )
}

export default function SecurityConfig() {
  const [tabValue, setTabValue] = useState(0)
  const [visitedTabs, setVisitedTabs] = useState([0])

  const handleTabChange = (_event: SyntheticEvent, newValue: string | number) => {
    const nextValue = Number(newValue)
    setTabValue(nextValue)
    setVisitedTabs((prev) => (prev.includes(nextValue) ? prev : [...prev, nextValue]))
  }

  return (
    <div className="w-full">
      <div className="border-b border-divider">
        <Tabs value={tabValue} onChange={handleTabChange} aria-label="安全配置">
          <Tab label="反主动探测" value={0} />
          <Tab label="会话绑定" value={1} />
          <Tab label="TLS 指纹伪装" value={2} />
          <Tab label="内生欺骗陷阱" value={3} />
        </Tabs>
      </div>

      <TabPanel value={tabValue} index={0} mounted={visitedTabs.includes(0)}>
        <AntiProbeConfigComponent />
      </TabPanel>

      <TabPanel value={tabValue} index={1} mounted={visitedTabs.includes(1)}>
        <SessionAffinityConfigComponent />
      </TabPanel>

      <TabPanel value={tabValue} index={2} mounted={visitedTabs.includes(2)}>
        <TlsFingerprintSelector />
      </TabPanel>

      <TabPanel value={tabValue} index={3} mounted={visitedTabs.includes(3)}>
        <SecurityMonitor />
      </TabPanel>
    </div>
  )
}
