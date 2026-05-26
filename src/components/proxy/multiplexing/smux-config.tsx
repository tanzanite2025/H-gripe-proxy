import {
  Box,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Switch,
  TextField,
  Typography,
} from '@mui/material'
import { useState } from 'react'

interface SmuxConfig {
  enabled: boolean
  protocol: 'smux' | 'yamux' | 'h2mux'
  'max-connections'?: number
  'min-streams'?: number
  'max-streams'?: number
  padding?: boolean
  statistic?: boolean
  'only-tcp'?: boolean
  'brutal-opts'?: {
    enabled: boolean
    up?: string
    down?: string
  }
}

interface SmuxConfigProps {
  config: SmuxConfig
  onChange: (config: SmuxConfig) => void
}

export function SmuxConfigComponent({ config, onChange }: SmuxConfigProps) {
  const [localConfig, setLocalConfig] = useState<SmuxConfig>(config)

  const handleChange = (updates: Partial<SmuxConfig>) => {
    const newConfig = { ...localConfig, ...updates }
    setLocalConfig(newConfig)
    onChange(newConfig)
  }

  const handleBrutalChange = (updates: Partial<SmuxConfig['brutal-opts']>) => {
    const newBrutalOpts = { ...localConfig['brutal-opts'], ...updates }
    handleChange({ 'brutal-opts': newBrutalOpts as any })
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Typography variant="h6">SMUX 多路复用配置</Typography>

      <FormControlLabel
        control={
          <Switch
            checked={localConfig.enabled}
            onChange={(e) => handleChange({ enabled: e.target.checked })}
          />
        }
        label="启用 SMUX"
      />

      {localConfig.enabled && (
        <>
          <FormControl fullWidth>
            <InputLabel>协议</InputLabel>
            <Select
              value={localConfig.protocol}
              label="协议"
              onChange={(e) =>
                handleChange({ protocol: e.target.value as any })
              }
            >
              <MenuItem value="smux">SMUX</MenuItem>
              <MenuItem value="yamux">Yamux</MenuItem>
              <MenuItem value="h2mux">H2Mux</MenuItem>
            </Select>
          </FormControl>

          <TextField
            label="最大连接数"
            type="number"
            value={localConfig['max-connections'] || ''}
            onChange={(e) =>
              handleChange({
                'max-connections': e.target.value
                  ? parseInt(e.target.value)
                  : undefined,
              })
            }
            helperText="默认值由协议决定"
          />

          <TextField
            label="最小流数"
            type="number"
            value={localConfig['min-streams'] || ''}
            onChange={(e) =>
              handleChange({
                'min-streams': e.target.value
                  ? parseInt(e.target.value)
                  : undefined,
              })
            }
            helperText="每个连接的最小流数"
          />

          <TextField
            label="最大流数"
            type="number"
            value={localConfig['max-streams'] || ''}
            onChange={(e) =>
              handleChange({
                'max-streams': e.target.value
                  ? parseInt(e.target.value)
                  : undefined,
              })
            }
            helperText="每个连接的最大流数，0 表示无限制"
          />

          <FormControlLabel
            control={
              <Switch
                checked={localConfig.padding || false}
                onChange={(e) => handleChange({ padding: e.target.checked })}
              />
            }
            label="启用填充"
          />

          <FormControlLabel
            control={
              <Switch
                checked={localConfig.statistic || false}
                onChange={(e) => handleChange({ statistic: e.target.checked })}
              />
            }
            label="启用统计"
          />

          <FormControlLabel
            control={
              <Switch
                checked={localConfig['only-tcp'] || false}
                onChange={(e) => handleChange({ 'only-tcp': e.target.checked })}
              />
            }
            label="仅 TCP"
          />

          <Box sx={{ mt: 2 }}>
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              Brutal 优化
            </Typography>

            <FormControlLabel
              control={
                <Switch
                  checked={localConfig['brutal-opts']?.enabled || false}
                  onChange={(e) =>
                    handleBrutalChange({ enabled: e.target.checked })
                  }
                />
              }
              label="启用 Brutal"
            />

            {localConfig['brutal-opts']?.enabled && (
              <Box sx={{ display: 'flex', gap: 2, mt: 1 }}>
                <TextField
                  label="上传速度"
                  value={localConfig['brutal-opts']?.up || ''}
                  onChange={(e) => handleBrutalChange({ up: e.target.value })}
                  helperText="例如: 100 Mbps"
                  fullWidth
                />
                <TextField
                  label="下载速度"
                  value={localConfig['brutal-opts']?.down || ''}
                  onChange={(e) => handleBrutalChange({ down: e.target.value })}
                  helperText="例如: 200 Mbps"
                  fullWidth
                />
              </Box>
            )}
          </Box>
        </>
      )}
    </Box>
  )
}
