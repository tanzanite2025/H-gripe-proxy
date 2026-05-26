import {
  Box,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Radio,
  RadioGroup,
  Select,
  Switch,
  TextField,
  Typography,
} from '@mui/material'
import { useState } from 'react'

type SudokuHttpMaskMode = 'legacy' | 'stream' | 'poll' | 'auto' | 'ws'
type SudokuHttpMaskMultiplex = 'off' | 'auto' | 'on'

interface SudokuHttpMaskConfig {
  disable?: boolean
  mode?: SudokuHttpMaskMode
  tls?: boolean
  host?: string
  'path-root'?: string
  multiplex?: SudokuHttpMaskMultiplex
}

interface SudokuMultiplexConfigProps {
  config: SudokuHttpMaskConfig
  onChange: (config: SudokuHttpMaskConfig) => void
}

const multiplexModes = [
  {
    value: 'off' as const,
    label: '关闭',
    description: '不使用 HTTP Mask 多路复用',
  },
  {
    value: 'auto' as const,
    label: '自动',
    description: '根据情况自动启用',
  },
  {
    value: 'on' as const,
    label: '开启',
    description: '始终使用 HTTP Mask 多路复用',
  },
]

const httpMaskModes = [
  { value: 'legacy' as const, label: 'Legacy' },
  { value: 'stream' as const, label: 'Stream' },
  { value: 'poll' as const, label: 'Poll' },
  { value: 'auto' as const, label: 'Auto' },
  { value: 'ws' as const, label: 'WebSocket' },
]

export function SudokuMultiplexConfig({
  config,
  onChange,
}: SudokuMultiplexConfigProps) {
  const [localConfig, setLocalConfig] =
    useState<SudokuHttpMaskConfig>(config)

  const handleChange = (updates: Partial<SudokuHttpMaskConfig>) => {
    const newConfig = { ...localConfig, ...updates }
    setLocalConfig(newConfig)
    onChange(newConfig)
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Typography variant="h6">Sudoku HTTP Mask 配置</Typography>

      <FormControlLabel
        control={
          <Switch
            checked={!localConfig.disable}
            onChange={(e) => handleChange({ disable: !e.target.checked })}
          />
        }
        label="启用 HTTP Mask"
      />

      {!localConfig.disable && (
        <>
          <FormControl fullWidth>
            <InputLabel>HTTP Mask 模式</InputLabel>
            <Select
              value={localConfig.mode || 'auto'}
              label="HTTP Mask 模式"
              onChange={(e) =>
                handleChange({ mode: e.target.value as SudokuHttpMaskMode })
              }
            >
              {httpMaskModes.map((mode) => (
                <MenuItem key={mode.value} value={mode.value}>
                  {mode.label}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          <FormControlLabel
            control={
              <Switch
                checked={localConfig.tls || false}
                onChange={(e) => handleChange({ tls: e.target.checked })}
              />
            }
            label="启用 TLS"
          />

          <TextField
            label="主机名"
            value={localConfig.host || ''}
            onChange={(e) => handleChange({ host: e.target.value })}
            helperText="HTTP Mask 使用的主机名"
            fullWidth
          />

          <TextField
            label="路径根"
            value={localConfig['path-root'] || ''}
            onChange={(e) => handleChange({ 'path-root': e.target.value })}
            helperText="HTTP Mask 使用的路径根"
            fullWidth
          />

          <Box sx={{ mt: 2 }}>
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              多路复用设置
            </Typography>

            <FormControl fullWidth>
              <InputLabel>多路复用模式</InputLabel>
              <Select
                value={localConfig.multiplex || 'auto'}
                label="多路复用模式"
                onChange={(e) =>
                  handleChange({
                    multiplex: e.target.value as SudokuHttpMaskMultiplex,
                  })
                }
              >
                {multiplexModes.map((mode) => (
                  <MenuItem key={mode.value} value={mode.value}>
                    <Box>
                      <Typography variant="body2">{mode.label}</Typography>
                      <Typography variant="caption" color="text.secondary">
                        {mode.description}
                      </Typography>
                    </Box>
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            <RadioGroup
              value={localConfig.multiplex || 'auto'}
              onChange={(e) =>
                handleChange({
                  multiplex: e.target.value as SudokuHttpMaskMultiplex,
                })
              }
              sx={{ mt: 1 }}
            >
              {multiplexModes.map((mode) => (
                <FormControlLabel
                  key={mode.value}
                  value={mode.value}
                  control={<Radio />}
                  label={
                    <Box>
                      <Typography variant="body2">{mode.label}</Typography>
                      <Typography variant="caption" color="text.secondary">
                        {mode.description}
                      </Typography>
                    </Box>
                  }
                />
              ))}
            </RadioGroup>
          </Box>
        </>
      )}
    </Box>
  )
}
