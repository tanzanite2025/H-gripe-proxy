import { Box, Tab, Tabs } from '@mui/material'
import { useState } from 'react'

import { MieruMultiplexConfig } from './mieru-multiplex-config'
import { SmuxConfigComponent } from './smux-config'
import { SudokuMultiplexConfig } from './sudoku-multiplex-config'

interface MultiplexingConfigProps {
  proxyType: string
  config: any
  onChange: (config: any) => void
}

export function MultiplexingConfig({
  proxyType,
  config,
  onChange,
}: MultiplexingConfigProps) {
  const [activeTab, setActiveTab] = useState(0)

  // 根据代理类型决定显示哪些配置
  const supportsSMUX = ['trojan', 'vmess', 'vless', 'ss'].includes(proxyType)
  const supportsMieru = proxyType === 'mieru'
  const supportsSudoku = proxyType === 'sudoku'

  if (!supportsSMUX && !supportsMieru && !supportsSudoku) {
    return (
      <Box sx={{ p: 2, textAlign: 'center', color: 'text.secondary' }}>
        该代理类型不支持多路复用配置
      </Box>
    )
  }

  return (
    <Box>
      {/* 如果支持多种多路复用，显示标签页 */}
      {(supportsSMUX && (supportsMieru || supportsSudoku)) && (
        <Tabs value={activeTab} onChange={(_, v) => setActiveTab(v)}>
          {supportsSMUX && <Tab label="SMUX" />}
          {supportsMieru && <Tab label="Mieru" />}
          {supportsSudoku && <Tab label="Sudoku" />}
        </Tabs>
      )}

      <Box sx={{ p: 2 }}>
        {/* SMUX 配置 */}
        {supportsSMUX && (!supportsMieru && !supportsSudoku || activeTab === 0) && (
          <SmuxConfigComponent
            config={config.smux || { enabled: false, protocol: 'yamux' }}
            onChange={(smuxConfig) => onChange({ ...config, smux: smuxConfig })}
          />
        )}

        {/* Mieru 配置 */}
        {supportsMieru && (!supportsSMUX || activeTab === (supportsSMUX ? 1 : 0)) && (
          <MieruMultiplexConfig
            multiplexing={config.multiplexing || 'MULTIPLEXING_MIDDLE'}
            onChange={(multiplexing) =>
              onChange({ ...config, multiplexing })
            }
          />
        )}

        {/* Sudoku 配置 */}
        {supportsSudoku && (
          <SudokuMultiplexConfig
            config={config.httpmask || { multiplex: 'auto' }}
            onChange={(httpmask) => onChange({ ...config, httpmask })}
          />
        )}
      </Box>
    </Box>
  )
}
