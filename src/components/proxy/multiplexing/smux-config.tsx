import { useState } from 'react'

import { Select } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'

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
    <div className="flex flex-col gap-4">
      <h3 className="text-lg font-semibold">SMUX 多路复用配置</h3>

      <label className="flex items-center gap-2">
        <Switch
          checked={localConfig.enabled}
          onChange={(e) => handleChange({ enabled: e.target.checked })}
        />
        <span>启用 SMUX</span>
      </label>

      {localConfig.enabled && (
        <>
          <Select
            value={localConfig.protocol}
            label="协议"
            onChange={(e) =>
              handleChange({ protocol: e.target.value as any })
            }
            fullWidth
          >
            <option value="smux">SMUX</option>
            <option value="yamux">Yamux</option>
            <option value="h2mux">H2Mux</option>
          </Select>

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

          <label className="flex items-center gap-2">
            <Switch
              checked={localConfig.padding || false}
              onChange={(e) => handleChange({ padding: e.target.checked })}
            />
            <span>启用填充</span>
          </label>

          <label className="flex items-center gap-2">
            <Switch
              checked={localConfig.statistic || false}
              onChange={(e) => handleChange({ statistic: e.target.checked })}
            />
            <span>启用统计</span>
          </label>

          <label className="flex items-center gap-2">
            <Switch
              checked={localConfig['only-tcp'] || false}
              onChange={(e) => handleChange({ 'only-tcp': e.target.checked })}
            />
            <span>仅 TCP</span>
          </label>

          <div className="mt-4">
            <h4 className="text-sm font-medium mb-2">Brutal 优化</h4>

            <label className="flex items-center gap-2 mb-2">
              <Switch
                checked={localConfig['brutal-opts']?.enabled || false}
                onChange={(e) =>
                  handleBrutalChange({ enabled: e.target.checked })
                }
              />
              <span>启用 Brutal</span>
            </label>

            {localConfig['brutal-opts']?.enabled && (
              <div className="flex gap-4 mt-2">
                <TextField
                  label="上传速度"
                  value={localConfig['brutal-opts']?.up || ''}
                  onChange={(e) => handleBrutalChange({ up: e.target.value })}
                  helperText="例如: 100 Mbps"
                  className="flex-1"
                />
                <TextField
                  label="下载速度"
                  value={localConfig['brutal-opts']?.down || ''}
                  onChange={(e) => handleBrutalChange({ down: e.target.value })}
                  helperText="例如: 200 Mbps"
                  className="flex-1"
                />
              </div>
            )}
          </div>
        </>
      )}
    </div>
  )
}
