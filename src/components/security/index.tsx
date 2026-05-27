/**
 * 安全配置主组件
 */

import { Tabs, Tab } from '@/components/tailwind'
import { useState } from 'react'

import AntiProbeConfigComponent from './anti-probe-config'
import TlsFingerprintSelector from './tls-fingerprint-selector'
import SecurityMonitor from './security-monitor'

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
      id={`security-tabpanel-${index}`}
      aria-labelledby={`security-tab-${index}`}
      {...other}
    >
      {value === index && <div>{children}</div>}
    </div>
  )
}

export default function SecurityConfig() {
  const [tabValue, setTabValue] = useState(0)

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue)
  }

  return (
    <div className="w-full">
      <div className="border-b border-divider">
        <Tabs value={tabValue} onChange={handleTabChange} aria-label="安全配置">
          <Tab label="反主动探测" value={0} />
          <Tab label="TLS 指纹伪装" value={1} />
          <Tab label="内生欺骗陷阱" value={2} />
        </Tabs>
      </div>

      <TabPanel value={tabValue} index={0}>
        <AntiProbeConfigComponent />
      </TabPanel>

      <TabPanel value={tabValue} index={1}>
        <TlsFingerprintSelector />
      </TabPanel>

      <TabPanel value={tabValue} index={2}>
        <SecurityMonitor />
      </TabPanel>
    </div>
  )
}
