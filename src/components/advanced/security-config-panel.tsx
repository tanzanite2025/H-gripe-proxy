/**
 * 安全防御配置面板
 */

import { Info } from 'lucide-react'
import { useState } from 'react'
import type { ChangeEvent } from 'react'

import { Switch, TextField } from '@/components/tailwind'
import type { SecurityConfig } from '@/services/coordinator'
import { tlsFingerprintGetAll } from '@/services/tls-fingerprint'

interface Props {
  config: SecurityConfig
  onChange: (config: SecurityConfig) => void
}

export function SecurityConfigPanel({ config, onChange }: Props) {
  const [fingerprints, setFingerprints] = useState<string[]>([])

  // 加载 TLS 指纹列表
  useState(() => {
    tlsFingerprintGetAll().then((fps) => {
      setFingerprints(fps.map((f) => f.name))
    })
  })

  return (
    <div>
      <div className="p-4 bg-blue-500 text-white rounded-lg mb-4">
        <div className="flex items-start gap-2">
          <Info className="w-5 h-5 flex-shrink-0 mt-0.5" />
          <p className="text-sm">
            安全防御功能可以保护您的代理免受主动探测和恶意扫描。
          </p>
        </div>
      </div>

      {/* 总开关 */}
      <div className="p-4 bg-card border border-border rounded-lg mb-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="font-semibold">启用安全监控</p>
            <p className="text-xs text-muted-foreground">
              启用反调试检测和内存蜜罐
            </p>
          </div>
          <Switch
            checked={config.enabled}
            onCheckedChange={(checked) =>
              onChange({ ...config, enabled: checked })
            }
          />
        </div>
      </div>

      {/* 反主动探测 */}
      <div className="p-4 bg-card border border-border rounded-lg mb-4">
        <h3 className="text-lg font-semibold mb-2">反主动探测</h3>
        <p className="text-sm text-muted-foreground mb-4">
          防止 GFW 等审查系统主动探测您的代理服务器
        </p>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium">启用反探测</label>
            <Switch
              checked={config.anti_probe.enabled}
              onCheckedChange={(checked) =>
                onChange({
                  ...config,
                  anti_probe: {
                    ...config.anti_probe,
                    enabled: checked,
                  },
                })
              }
            />
          </div>

          {config.anti_probe.enabled && (
            <>
              <TextField
                label="时间窗口（秒）"
                type="number"
                value={config.anti_probe.time_window.toString()}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  onChange({
                    ...config,
                    anti_probe: {
                      ...config.anti_probe,
                      time_window: Number.parseInt(e.target.value) || 300,
                    },
                  })
                }
                helperText="握手暗号的有效时间"
                fullWidth
              />

              <div className="flex items-start justify-between">
                <div>
                  <p className="text-sm font-medium">严格模式</p>
                  <p className="text-xs text-muted-foreground">
                    非白名单 IP 直接拒绝连接
                  </p>
                </div>
                <Switch
                  checked={config.anti_probe.strict_mode}
                  onCheckedChange={(checked) =>
                    onChange({
                      ...config,
                      anti_probe: {
                        ...config.anti_probe,
                        strict_mode: checked,
                      },
                    })
                  }
                />
              </div>

              <div>
                <p className="text-sm font-medium mb-1">白名单 IP</p>
                <p className="text-xs text-muted-foreground">
                  这些 IP 可以直接连接，无需验证
                </p>
                {/* TODO: 添加 IP 列表编辑器 */}
              </div>
            </>
          )}
        </div>
      </div>

      {/* TLS 指纹伪装 */}
      <div className="p-4 bg-card border border-border rounded-lg mb-4">
        <h3 className="text-lg font-semibold mb-2">TLS 指纹伪装</h3>
        <p className="text-sm text-muted-foreground mb-4">
          伪装成常见浏览器或应用的 TLS 指纹
        </p>

        <div className="space-y-4">
          <div>
            <p className="text-sm font-medium mb-2">选择指纹</p>
            <div className="flex flex-wrap gap-2">
              <button
                onClick={() => onChange({ ...config, tls_fingerprint: null })}
                className={`px-3 py-1 rounded-full text-sm ${
                  !config.tls_fingerprint
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-secondary text-secondary-foreground'
                }`}
              >
                不使用
              </button>
              {fingerprints.map((name) => (
                <button
                  key={name}
                  onClick={() =>
                    onChange({ ...config, tls_fingerprint: name })
                  }
                  className={`px-3 py-1 rounded-full text-sm ${
                    config.tls_fingerprint === name
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-secondary text-secondary-foreground'
                  }`}
                >
                  {name}
                </button>
              ))}
            </div>
          </div>

          {config.tls_fingerprint && (
            <div className="p-3 bg-green-500 text-white rounded-lg">
              <p className="text-sm">当前使用：{config.tls_fingerprint}</p>
            </div>
          )}
        </div>
      </div>

      {/* 配置欺骗 */}
      <div className="p-4 bg-card border border-border rounded-lg">
        <h3 className="text-lg font-semibold mb-2">配置欺骗</h3>
        <p className="text-sm text-muted-foreground mb-4">
          创建假配置文件误导扫描工具
        </p>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium">启用配置欺骗</label>
            <Switch
              checked={config.config_decoy.enabled}
              onCheckedChange={(checked) =>
                onChange({
                  ...config,
                  config_decoy: {
                    ...config.config_decoy,
                    enabled: checked,
                  },
                })
              }
            />
          </div>

          {config.config_decoy.enabled && (
            <div className="p-3 bg-yellow-500 text-white rounded-lg">
              <p className="text-sm">
                真实配置将被加密存储，假配置将放置在明显位置
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
