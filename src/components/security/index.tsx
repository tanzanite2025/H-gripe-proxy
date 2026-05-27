/**
 * 安全配置主组件
 */

import { Box, Tab, Tabs } from '@mui/material'
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
      {value === index && <Box>{children}</Box>}
    </div>
  )
}

export default function SecurityConfig() {
  const [tabValue, setTabValue] = useState(0)

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue)
  }

  return (
    <Box sx={{ width: '100%' }}>
      <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
        <Tabs value={tabValue} onChange={handleTabChange} aria-label="安全配置">
          <Tab label="反主动探测" />
          <Tab label="TLS 指纹伪装" />
          <Tab label="内生欺骗陷阱" />
        </Tabs>
      </Box>

      <TabPanel value={tabValue} index={0}>
        <AntiProbeConfigComponent />
      </TabPanel>

      <TabPanel value={tabValue} index={1}>
        <TlsFingerprintSelector />
      </TabPanel>

      <TabPanel value={tabValue} index={2}>
        <SecurityMonitor />
      </TabPanel>
    </Box>
  )
}
