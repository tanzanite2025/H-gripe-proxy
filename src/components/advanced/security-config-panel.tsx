/**
 * 安全防御配置面板
 */

import { Info } from 'lucide-react'
import { useState } from 'react'
import type { ChangeEvent } from 'react'

import { Switch, TextField } from '@/components/tailwind'
import type { SecurityConfig } from '@/services/coordinator'
import { tlsFingerprintGetAll, type TlsFingerprint } from '@/services/tls-fingerprint'

interface Props {
  config: SecurityConfig
  onChange: (config: SecurityConfig) => void
}

export function SecurityConfigPanel({ config, onChange }: Props) {
  const [fingerprints, setFingerprints] = useState<TlsFingerprint[]>([])

  // 加载 TLS 指纹列表
  useState(() => {
    tlsFingerprintGetAll().then((fps) => {
      setFingerprints(fps)
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
              {['browser', 'mobile', 'random', 'classic'].map((cat) => (
                <div key={cat}>
                  <p className="text-xs text-muted-foreground mt-2 mb-1">
                    {cat === 'browser' ? '浏览器' : cat === 'mobile' ? '移动端' : cat === 'random' ? '随机' : '经典'}
                  </p>
                  <div className="flex flex-wrap gap-2">
                    {fingerprints
                      .filter((f) => f.category === cat)
                      .map((fp) => (
                        <button
                          key={fp.name}
                          onClick={() =>
                            onChange({ ...config, tls_fingerprint: fp.name })
                          }
                          className={`px-3 py-1 rounded-full text-sm ${
                            config.tls_fingerprint === fp.name
                              ? 'bg-primary text-primary-foreground'
                              : 'bg-secondary text-secondary-foreground'
                          }`}
                        >
                          {fp.description}
                        </button>
                      ))}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {config.tls_fingerprint && (
            <div className="p-3 bg-green-500 text-white rounded-lg">
                当前使用：{fingerprints.find((f) => f.name === config.tls_fingerprint)?.description ?? config.tls_fingerprint}
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

      {/* Sniffer 嗅探 */}
      <div className="p-4 bg-card border border-border rounded-lg mt-4">
        <h3 className="text-lg font-semibold mb-2">流量嗅探</h3>
        <p className="text-sm text-muted-foreground mb-4">
          从加密流量中提取域名，使安全策略能基于域名匹配
        </p>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">启用嗅探</p>
              <p className="text-xs text-muted-foreground">
                从 TLS/HTTP/QUIC 流量中提取 SNI/Host 域名
              </p>
            </div>
            <Switch
              checked={config.sniffer.enabled}
              onCheckedChange={(checked) =>
                onChange({
                  ...config,
                  sniffer: { ...config.sniffer, enabled: checked },
                })
              }
            />
          </div>

          {config.sniffer.enabled && (
            <>
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium">覆盖目标地址</p>
                  <p className="text-xs text-muted-foreground">
                    用嗅探到的域名替换原始目标 IP
                  </p>
                </div>
                <Switch
                  checked={config.sniffer.overrideDest}
                  onCheckedChange={(checked) =>
                    onChange({
                      ...config,
                      sniffer: { ...config.sniffer, overrideDest: checked },
                    })
                  }
                />
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium">解析纯 IP 连接</p>
                  <p className="text-xs text-muted-foreground">
                    对无域名的纯 IP 连接执行嗅探
                  </p>
                </div>
                <Switch
                  checked={config.sniffer.parsePureIp}
                  onCheckedChange={(checked) =>
                    onChange({
                      ...config,
                      sniffer: { ...config.sniffer, parsePureIp: checked },
                    })
                  }
                />
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium">强制 DNS 映射</p>
                  <p className="text-xs text-muted-foreground">
                    对 DNS 映射结果强制执行嗅探
                  </p>
                </div>
                <Switch
                  checked={config.sniffer.forceDnsMapping}
                  onCheckedChange={(checked) =>
                    onChange({
                      ...config,
                      sniffer: { ...config.sniffer, forceDnsMapping: checked },
                    })
                  }
                />
              </div>

              <div>
                <p className="text-sm font-medium mb-2">嗅探类型</p>
                <div className="flex flex-wrap gap-2">
                  {['TLS', 'HTTP', 'QUIC'].map((type) => (
                    <button
                      key={type}
                      onClick={() => {
                        const sniffing = config.sniffer.sniffing.includes(type)
                          ? config.sniffer.sniffing.filter((t) => t !== type)
                          : [...config.sniffer.sniffing, type]
                        onChange({
                          ...config,
                          sniffer: { ...config.sniffer, sniffing },
                        })
                      }}
                      className={`px-3 py-1 rounded-full text-sm `}
                    >
                      {type}
                    </button>
                  ))}
                </div>
              </div>

              <TextField
                label="强制嗅探域名（逗号分隔）"
                value={config.sniffer.forceDomain.join(', ')}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  onChange({
                    ...config,
                    sniffer: {
                      ...config.sniffer,
                      forceDomain: e.target.value
                        .split(',')
                        .map((s) => s.trim())
                        .filter(Boolean),
                    },
                  })
                }
                helperText="这些域名将强制执行嗅探"
                fullWidth
              />

              <TextField
                label="跳过嗅探域名（逗号分隔）"
                value={config.sniffer.skipDomain.join(', ')}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  onChange({
                    ...config,
                    sniffer: {
                      ...config.sniffer,
                      skipDomain: e.target.value
                        .split(',')
                        .map((s) => s.trim())
                        .filter(Boolean),
                    },
                  })
                }
                helperText="这些域名将跳过嗅探"
                fullWidth
              />
            </>
          )}
        </div>
      </div>

      {/* 流量混淆 */}
      <div className="p-4 bg-card border border-border rounded-lg mt-4">
        <h3 className="text-lg font-semibold mb-2">流量混淆</h3>
        <p className="text-sm text-muted-foreground mb-4">
          混淆流量特征，防止流量分析和指纹识别
        </p>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium">启用混淆</label>
            <Switch
              checked={config.obfuscation.enabled}
              onCheckedChange={(checked) =>
                onChange({
                  ...config,
                  obfuscation: {
                    ...config.obfuscation,
                    enabled: checked,
                  },
                })
              }
            />
          </div>

          {config.obfuscation.enabled && (
            <>
              <div>
                <p className="text-sm font-medium mb-2">混淆级别</p>
                <div className="flex flex-wrap gap-2">
                  {(['low', 'medium', 'high', 'paranoid'] as const).map(
                    (lvl) => (
                      <button
                        key={lvl}
                        onClick={() =>
                          onChange({
                            ...config,
                            obfuscation: {
                              ...config.obfuscation,
                              level: lvl,
                            },
                          })
                        }
                        className={`px-3 py-1 rounded-full text-sm ${
                          config.obfuscation.level === lvl
                            ? 'bg-primary text-primary-foreground'
                            : 'bg-secondary text-secondary-foreground'
                        }`}
                      >
                        {lvl === 'low'
                          ? '低级'
                          : lvl === 'medium'
                            ? '中级'
                            : lvl === 'high'
                              ? '高级'
                              : '偏执'}
                      </button>
                    ),
                  )}
                </div>
              </div>

              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">自动调整</label>
                <Switch
                  checked={config.obfuscation.autoAdjust}
                  onCheckedChange={(checked) =>
                    onChange({
                      ...config,
                      obfuscation: {
                        ...config.obfuscation,
                        autoAdjust: checked,
                      },
                    })
                  }
                />
              </div>

              {config.obfuscation.level === 'high' ||
              config.obfuscation.level === 'paranoid' ? (
                <div className="p-3 bg-orange-500 text-white rounded-lg">
                  <p className="text-sm">
                    {config.obfuscation.level === 'high'
                      ? '高级混淆：TLS 指纹将自动切换为随机模式'
                      : '偏执混淆：TLS 指纹将使用完全随机化模式，性能影响较大'}
                  </p>
                </div>
              ) : null}
            </>
          )}
        </div>
      </div>
      </div>

    </div>
  )
}
