import { Shield } from 'lucide-react'
import { useEffect, useMemo, useState, type ChangeEvent } from 'react'

import { Alert, Chip, TextField } from '@/components/tailwind'
import type {
  ObfuscationConfig,
  SecurityConfig,
  SnifferConfig,
} from '@/services/coordinator'
import {
  tlsFingerprintGetAll,
  type TlsFingerprint,
} from '@/services/tls-fingerprint'

import {
  DEFAULT_FINGERPRINT_CATEGORY_ORDER,
  FINGERPRINT_CATEGORY_LABELS,
  OBFUSCATION_LEVEL_OPTIONS,
  SNIFFING_TYPES,
} from './constants'
import { ChoiceButton, SectionCard, ToggleRow } from './shared'

interface Props {
  config: SecurityConfig
  onChange: (config: SecurityConfig) => void
}

function parseStringList(value: string) {
  return value
    .split(/[\n,]+/)
    .map((item) => item.trim())
    .filter(Boolean)
}

function parseInteger(value: string, fallback: number) {
  const parsed = Number.parseInt(value, 10)
  return Number.isFinite(parsed) ? parsed : fallback
}

export function SecurityConfigPanel({ config, onChange }: Props) {
  const [fingerprints, setFingerprints] = useState<TlsFingerprint[]>([])
  const [fingerprintsLoading, setFingerprintsLoading] = useState(true)
  const [fingerprintsError, setFingerprintsError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    const loadFingerprints = async () => {
      setFingerprintsLoading(true)
      setFingerprintsError(null)

      try {
        const nextFingerprints = await tlsFingerprintGetAll()
        if (!cancelled) {
          setFingerprints(nextFingerprints)
        }
      } catch (error) {
        if (!cancelled) {
          setFingerprints([])
          setFingerprintsError(String(error))
        }
      } finally {
        if (!cancelled) {
          setFingerprintsLoading(false)
        }
      }
    }

    void loadFingerprints()

    return () => {
      cancelled = true
    }
  }, [])

  const updateConfig = <K extends keyof SecurityConfig>(
    key: K,
    value: SecurityConfig[K],
  ) => {
    onChange({ ...config, [key]: value })
  }

  const updateSniffer = <K extends keyof SnifferConfig>(
    key: K,
    value: SnifferConfig[K],
  ) => {
    updateConfig('sniffer', { ...config.sniffer, [key]: value })
  }

  const updateObfuscation = <K extends keyof ObfuscationConfig>(
    key: K,
    value: ObfuscationConfig[K],
  ) => {
    updateConfig('obfuscation', { ...config.obfuscation, [key]: value })
  }

  const fingerprintCategories = useMemo(() => {
    const dynamicCategories = Array.from(
      new Set(fingerprints.map((fingerprint) => fingerprint.category)),
    ).filter((category) => !DEFAULT_FINGERPRINT_CATEGORY_ORDER.includes(category))

    return [...DEFAULT_FINGERPRINT_CATEGORY_ORDER, ...dynamicCategories]
      .map((category) => ({
        key: category,
        label: FINGERPRINT_CATEGORY_LABELS[category] ?? category,
        items: fingerprints.filter(
          (fingerprint) => fingerprint.category === category,
        ),
      }))
      .filter((category) => category.items.length > 0)
  }, [fingerprints])

  const selectedFingerprint =
    fingerprints.find(
      (fingerprint) => fingerprint.name === config.tls_fingerprint,
    ) ?? null

  const toggleSniffingType = (type: (typeof SNIFFING_TYPES)[number]) => {
    const nextSniffing = config.sniffer.sniffing.includes(type)
      ? config.sniffer.sniffing.filter((item) => item !== type)
      : [...config.sniffer.sniffing, type]

    updateSniffer('sniffing', nextSniffing)
  }

  return (
    <div className="space-y-4">
      <Alert severity="info" className="text-sm">
        安全防护用于减少主动探测、特征识别和配置暴露带来的风险。顶部总开关决定这些能力是否生效，下面各项配置会被保留，方便按场景逐项启用。
      </Alert>

      <section className="rounded-lg border border-border bg-card p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 text-lg font-semibold">
              <Shield className="h-4 w-4" />
              安全防护总开关
            </div>
            <p className="mt-1 text-sm text-muted-foreground">
              关闭后不会清空各项子配置，只是暂时不让它们参与运行。
            </p>
          </div>

          <div className="flex items-center gap-2">
            <Chip
              size="small"
              color={config.enabled ? 'success' : 'default'}
              label={config.enabled ? '已启用' : '未启用'}
            />
          </div>
        </div>

        <div className="mt-4 flex items-center justify-between gap-3 rounded-lg border border-border px-4 py-3">
          <div>
            <p className="text-sm font-medium">启用安全防护</p>
            <p className="mt-1 text-xs text-muted-foreground">
              统一控制反主动探测、TLS 指纹、流量嗅探和混淆等能力是否参与运行。
            </p>
          </div>
          <ToggleRow
            title=""
            checked={config.enabled}
            onCheckedChange={(checked) => updateConfig('enabled', checked)}
          />
        </div>
      </section>

      <SectionCard
        title="反主动探测"
        description="用于降低被探测端口、被回放握手或被异常流量扫描识别的概率。"
      >
        <ToggleRow
          title="启用反主动探测"
          description="开启后会启用握手校验、时间窗口限制和可选的严格访问控制。"
          checked={config.anti_probe.enabled}
          onCheckedChange={(checked) =>
            updateConfig('anti_probe', {
              ...config.anti_probe,
              enabled: checked,
            })
          }
        />

        {config.anti_probe.enabled ? (
          <>
            <TextField
              label="握手密钥"
              value={config.anti_probe.secret_key}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                updateConfig('anti_probe', {
                  ...config.anti_probe,
                  secret_key: event.target.value,
                })
              }
              helperText="建议填写只用于该环境的独立密钥，避免与其他场景复用。"
              fullWidth
            />

            <TextField
              label="时间窗口（秒）"
              type="number"
              value={String(config.anti_probe.time_window)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                updateConfig('anti_probe', {
                  ...config.anti_probe,
                  time_window: parseInteger(
                    event.target.value,
                    config.anti_probe.time_window,
                  ),
                })
              }
              helperText="握手校验允许的时间偏移窗口。"
              fullWidth
            />

            <ToggleRow
              title="严格模式"
              description="未命中白名单的来源将优先被拒绝，更适合固定入口或自控环境。"
              checked={config.anti_probe.strict_mode}
              onCheckedChange={(checked) =>
                updateConfig('anti_probe', {
                  ...config.anti_probe,
                  strict_mode: checked,
                })
              }
            />

            <TextField
              label="白名单来源"
              value={config.anti_probe.whitelist.join('\n')}
              onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                updateConfig('anti_probe', {
                  ...config.anti_probe,
                  whitelist: parseStringList(event.target.value),
                })
              }
              helperText="支持逗号或换行分隔，适合填固定 IP、网段或内部出口。"
              multiline
              rows={3}
              fullWidth
            />
          </>
        ) : null}
      </SectionCard>

      <SectionCard
        title="TLS 指纹"
        description="通过模拟常见客户端的 TLS 指纹，减少出口握手特征过于固定带来的识别风险。"
        aside={
          config.tls_fingerprint ? (
            <Chip size="small" color="success" label="已选择" />
          ) : (
            <Chip size="small" color="default" label="未启用" />
          )
        }
      >
        {fingerprintsLoading ? (
          <Alert severity="info" className="text-sm">
            正在加载可用的 TLS 指纹列表。
          </Alert>
        ) : null}

        {fingerprintsError ? (
          <Alert severity="warning" className="text-sm">
            TLS 指纹列表加载失败：{fingerprintsError}
          </Alert>
        ) : null}

        <div>
          <p className="mb-2 text-sm font-medium">选择指纹</p>
          <div className="flex flex-wrap gap-2">
            <ChoiceButton
              active={!config.tls_fingerprint}
              onClick={() => updateConfig('tls_fingerprint', null)}
            >
              不使用
            </ChoiceButton>
          </div>
        </div>

        {fingerprintCategories.map((category) => (
          <div key={category.key}>
            <p className="mb-2 text-sm font-medium">{category.label}</p>
            <div className="flex flex-wrap gap-2">
              {category.items.map((fingerprint) => (
                <ChoiceButton
                  key={fingerprint.name}
                  active={config.tls_fingerprint === fingerprint.name}
                  onClick={() =>
                    updateConfig('tls_fingerprint', fingerprint.name)
                  }
                >
                  {fingerprint.description}
                </ChoiceButton>
              ))}
            </div>
          </div>
        ))}

        {config.tls_fingerprint ? (
          <Alert severity="success" className="text-sm">
            当前选择：{selectedFingerprint?.description ?? config.tls_fingerprint}
          </Alert>
        ) : null}
      </SectionCard>

      <SectionCard
        title="配置诱饵"
        description="用于把真实配置与对外可见的伪装配置分离，降低误扫或误取证时的暴露风险。"
      >
        <ToggleRow
          title="启用配置诱饵"
          description="启用后可以指定诱饵配置路径，用于暴露一份对外无害的替代配置。"
          checked={config.config_decoy.enabled}
          onCheckedChange={(checked) =>
            updateConfig('config_decoy', {
              ...config.config_decoy,
              enabled: checked,
            })
          }
        />

        {config.config_decoy.enabled ? (
          <>
            <TextField
              label="诱饵配置路径"
              value={config.config_decoy.decoy_path ?? ''}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                updateConfig('config_decoy', {
                  ...config.config_decoy,
                  decoy_path: event.target.value.trim() || null,
                })
              }
              helperText="可留空表示让后端按默认策略生成；填写时建议指向单独目录。"
              fullWidth
            />

            <Alert severity="warning" className="text-sm">
              诱饵配置应与真实运行配置隔离，避免同目录混放造成误删或误同步。
            </Alert>
          </>
        ) : null}
      </SectionCard>

      <SectionCard
        title="流量嗅探"
        description="从 TLS、HTTP 或 QUIC 流量中提取域名信息，便于安全策略和分流逻辑基于域名做判断。"
      >
        <ToggleRow
          title="启用嗅探"
          description="关闭后会保留当前参数，但不再解析出站流量中的 SNI 或 Host。"
          checked={config.sniffer.enabled}
          onCheckedChange={(checked) => updateSniffer('enabled', checked)}
        />

        {config.sniffer.enabled ? (
          <>
            <ToggleRow
              title="覆盖目标地址"
              description="嗅探到域名后，用域名替换原始目的地址，适合依赖域名规则的场景。"
              checked={config.sniffer.overrideDest}
              onCheckedChange={(checked) =>
                updateSniffer('overrideDest', checked)
              }
            />

            <ToggleRow
              title="解析纯 IP 连接"
              description="即使目标是纯 IP，也尝试从握手里提取额外特征。"
              checked={config.sniffer.parsePureIp}
              onCheckedChange={(checked) =>
                updateSniffer('parsePureIp', checked)
              }
            />

            <ToggleRow
              title="强制 DNS 映射"
              description="对 DNS 映射得到的结果也强制做一次嗅探，适合规则依赖域名的场景。"
              checked={config.sniffer.forceDnsMapping}
              onCheckedChange={(checked) =>
                updateSniffer('forceDnsMapping', checked)
              }
            />

            <div>
              <p className="mb-2 text-sm font-medium">嗅探类型</p>
              <div className="flex flex-wrap gap-2">
                {SNIFFING_TYPES.map((type) => (
                  <ChoiceButton
                    key={type}
                    active={config.sniffer.sniffing.includes(type)}
                    onClick={() => toggleSniffingType(type)}
                  >
                    {type}
                  </ChoiceButton>
                ))}
              </div>
            </div>

            <TextField
              label="强制嗅探域名"
              value={config.sniffer.forceDomain.join('\n')}
              onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                updateSniffer('forceDomain', parseStringList(event.target.value))
              }
              helperText="支持逗号或换行分隔；这些域名会始终执行嗅探。"
              multiline
              rows={3}
              fullWidth
            />

            <TextField
              label="跳过嗅探域名"
              value={config.sniffer.skipDomain.join('\n')}
              onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                updateSniffer('skipDomain', parseStringList(event.target.value))
              }
              helperText="支持逗号或换行分隔；这些域名会直接跳过嗅探。"
              multiline
              rows={3}
              fullWidth
            />
          </>
        ) : null}
      </SectionCard>

      <SectionCard
        title="流量混淆"
        description="通过调整特征、节奏和指纹暴露程度，降低出口行为过于稳定导致的可识别性。"
      >
        <ToggleRow
          title="启用混淆"
          description="开启后可按级别控制混淆强度，并按需启用自动调节。"
          checked={config.obfuscation.enabled}
          onCheckedChange={(checked) => updateObfuscation('enabled', checked)}
        />

        {config.obfuscation.enabled ? (
          <>
            <div>
              <p className="mb-2 text-sm font-medium">混淆级别</p>
              <div className="flex flex-wrap gap-2">
                {OBFUSCATION_LEVEL_OPTIONS.map((option) => (
                  <ChoiceButton
                    key={option.value}
                    active={config.obfuscation.level === option.value}
                    onClick={() =>
                      updateObfuscation('level', option.value)
                    }
                  >
                    {option.label}
                  </ChoiceButton>
                ))}
              </div>
            </div>

            <ToggleRow
              title="自动调节"
              description="根据当前混淆级别和上下文动态调整参数，减少手工来回切换。"
              checked={config.obfuscation.autoAdjust}
              onCheckedChange={(checked) =>
                updateObfuscation('autoAdjust', checked)
              }
            />

            {config.obfuscation.level === 'high' ? (
              <Alert severity="warning" className="text-sm">
                高等级混淆通常会让 TLS 指纹更偏向动态策略，兼容性与性能都可能受到影响。
              </Alert>
            ) : null}

            {config.obfuscation.level === 'paranoid' ? (
              <Alert severity="warning" className="text-sm">
                偏执模式会进一步放大随机化程度，适合高对抗场景，但更容易带来吞吐和稳定性损耗。
              </Alert>
            ) : null}
          </>
        ) : null}
      </SectionCard>
    </div>
  )
}
