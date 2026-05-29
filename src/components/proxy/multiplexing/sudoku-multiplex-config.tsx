import { useState, type ChangeEvent } from 'react'

import { Radio, RadioGroup } from '@/components/tailwind/Radio'
import { Select, type SelectChangeEvent } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'

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
    <div className="flex flex-col gap-4">
      <h3 className="text-lg font-semibold">Sudoku HTTP Mask 配置</h3>

      <label className="flex items-center gap-2">
        <Switch
          checked={!localConfig.disable}
          onCheckedChange={(checked) => handleChange({ disable: !checked })}
        />
        <span>启用 HTTP Mask</span>
      </label>

      {!localConfig.disable && (
        <>
          <Select
            value={localConfig.mode || 'auto'}
            label="HTTP Mask 模式"
            onChange={(event: SelectChangeEvent) =>
              handleChange({ mode: event.target.value as SudokuHttpMaskMode })
            }
            fullWidth
          >
            {httpMaskModes.map((mode) => (
              <option key={mode.value} value={mode.value}>
                {mode.label}
              </option>
            ))}
          </Select>

          <label className="flex items-center gap-2">
            <Switch
              checked={localConfig.tls || false}
              onCheckedChange={(checked) => handleChange({ tls: checked })}
            />
            <span>启用 TLS</span>
          </label>

          <TextField
            label="主机名"
            value={localConfig.host || ''}
            onChange={(event: ChangeEvent<HTMLInputElement>) => handleChange({ host: event.target.value })}
            helperText="HTTP Mask 使用的主机名"
            className="w-full"
          />

          <TextField
            label="路径根"
            value={localConfig['path-root'] || ''}
            onChange={(event: ChangeEvent<HTMLInputElement>) => handleChange({ 'path-root': event.target.value })}
            helperText="HTTP Mask 使用的路径根"
            className="w-full"
          />

          <div className="mt-4">
            <h4 className="text-sm font-medium mb-2">多路复用设置</h4>

            <Select
              value={localConfig.multiplex || 'auto'}
              label="多路复用模式"
              onChange={(event: SelectChangeEvent) =>
                handleChange({
                  multiplex: event.target.value as SudokuHttpMaskMultiplex,
                })
              }
              fullWidth
            >
              {multiplexModes.map((mode) => (
                <option key={mode.value} value={mode.value}>
                  {mode.label} - {mode.description}
                </option>
              ))}
            </Select>

            <RadioGroup
              value={localConfig.multiplex || 'auto'}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                handleChange({
                  multiplex: event.target.value as SudokuHttpMaskMultiplex,
                })
              }
              className="mt-2"
            >
              {multiplexModes.map((mode) => (
                <label key={mode.value} className="flex items-start gap-2 mb-2">
                  <Radio value={mode.value} />
                  <div>
                    <div className="text-sm">{mode.label}</div>
                    <div className="text-xs text-gray-500">
                      {mode.description}
                    </div>
                  </div>
                </label>
              ))}
            </RadioGroup>
          </div>
        </>
      )}
    </div>
  )
}
