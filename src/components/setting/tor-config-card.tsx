/**
 * Tor 代理配置卡片
 */

import { useQuery, useQueryClient } from '@tanstack/react-query'
import { CheckCircle as CheckIcon, ChevronDown as ExpandMoreIcon, Copy as CopyIcon, AlertCircle as ErrorIcon, Shield as TorIcon } from 'lucide-react'
import { useMemo, useState } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { Collapse } from '@/components/tailwind/Collapse'
import { IconButton } from '@/components/tailwind/IconButton'
import { List, ListItem, ListItemText } from '@/components/tailwind/List'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import { useVerge } from '@/hooks/system'
import { getTorStatus, testTorConnection } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { cn } from '@/utils/cn'

const DEFAULT_TOR_CONFIG = {
  enabled: false,
  socksHost: '127.0.0.1',
  socksPort: 9050,
  controlPort: 9051,
  useBridges: false,
  bridges: [] as string[],
}

const torUsageInstructions = {
  title: 'Tor 使用说明',
  steps: [
    '1. 下载并安装 Tor Browser 或 Tor Expert Bundle',
    '2. 启动 Tor，确保 SOCKS5 代理运行在 127.0.0.1:9050',
    '3. 在 Clash Verge 中启用 Tor 代理',
    '4. 配置代理规则使用 Tor',
    '5. （可选）配置 DoH 防止 DNS 泄露',
  ],
  notes: [
    '• Tor 会显著降低网络速度（通常 < 1 Mbps）',
    '• 建议配合 DoH 使用，防止 DNS 泄露',
    '• 在某些地区可能需要使用网桥（Bridges）',
    '• 不要在 Tor 上进行大流量下载',
    '• 定期更换 Tor 电路以提高匿名性',
  ],
}

const buildTorSocksUrl = (host: string, port: number) => `socks5://${host}:${port}`

const getAssessmentLabel = (assessment?: string) => {
  switch (assessment) {
    case 'connected':
      return '已验证连通'
    case 'runtime-risk':
      return '存在运行风险'
    case 'inconclusive':
      return '结果不确定'
    case 'disabled':
      return '未启用'
    default:
      return assessment || '未知'
  }
}

const getAssessmentColor = (assessment?: string) => {
  switch (assessment) {
    case 'connected':
      return 'success' as const
    case 'runtime-risk':
      return 'warning' as const
    case 'inconclusive':
      return 'info' as const
    case 'disabled':
      return 'default' as const
    default:
      return 'default' as const
  }
}

const getConfidenceLabel = (confidence?: string) => {
  switch (confidence) {
    case 'high':
      return '高置信度'
    case 'medium':
      return '中置信度'
    case 'low':
      return '低置信度'
    default:
      return confidence || '未知'
  }
}

const formatRuntimeRiskLabel = (risk: string) => {
  switch (risk) {
    case 'non-local-socks-endpoint':
      return 'SOCKS 端点不是本机地址'
    case 'invalid-socks-port':
      return 'SOCKS 端口无效'
    case 'bridges-enabled-without-bridges':
      return '启用网桥但未配置网桥'
    default:
      return risk
  }
}

const parseBridgeList = (value: string) =>
  value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean)

export const TorConfigCard = () => {
  const { verge, patchVerge } = useVerge()
  const queryClient = useQueryClient()
  const [draftConfig, setDraftConfig] = useState<null | typeof DEFAULT_TOR_CONFIG>(null)
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [showInstructions, setShowInstructions] = useState(false)

  const persistedConfig = useMemo(
    () => ({
      enabled: verge?.enable_tor_proxy ?? DEFAULT_TOR_CONFIG.enabled,
      socksHost: verge?.tor_socks_host ?? DEFAULT_TOR_CONFIG.socksHost,
      socksPort: verge?.tor_socks_port ?? DEFAULT_TOR_CONFIG.socksPort,
      controlPort: verge?.tor_control_port ?? DEFAULT_TOR_CONFIG.controlPort,
      useBridges: verge?.tor_use_bridges ?? DEFAULT_TOR_CONFIG.useBridges,
      bridges: verge?.tor_bridges ?? DEFAULT_TOR_CONFIG.bridges,
    }),
    [
      verge?.enable_tor_proxy,
      verge?.tor_socks_host,
      verge?.tor_socks_port,
      verge?.tor_control_port,
      verge?.tor_use_bridges,
      verge?.tor_bridges,
    ],
  )

  const currentConfig = draftConfig ?? persistedConfig

  const torStatusQueryKey = useMemo(
    () => [
      'tor-status',
      persistedConfig.enabled,
      persistedConfig.socksHost,
      persistedConfig.socksPort,
      persistedConfig.controlPort,
      persistedConfig.useBridges,
      persistedConfig.bridges.join('|'),
    ],
    [
      persistedConfig.enabled,
      persistedConfig.socksHost,
      persistedConfig.socksPort,
      persistedConfig.controlPort,
      persistedConfig.useBridges,
      persistedConfig.bridges,
    ],
  )

  const {
    data: status,
    isLoading: statusLoading,
    isFetching: statusFetching,
    refetch: refetchStatus,
  } = useQuery({
    queryKey: torStatusQueryKey,
    queryFn: getTorStatus,
    enabled: !!verge,
    refetchOnWindowFocus: false,
    retry: false,
  })

  const saveTorConfig = async (nextConfig: typeof DEFAULT_TOR_CONFIG) => {
    setSaving(true)

    try {
      await patchVerge({
        enable_tor_proxy: nextConfig.enabled,
        tor_socks_host: nextConfig.socksHost,
        tor_socks_port: nextConfig.socksPort,
        tor_control_port: nextConfig.controlPort,
        tor_use_bridges: nextConfig.useBridges,
        tor_bridges: nextConfig.bridges,
      })
      setDraftConfig(null)
      await refetchStatus()
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSaving(false)
    }
  }

  const handleEnableChange = (checked: boolean) => {
    void saveTorConfig({
      ...currentConfig,
      enabled: checked,
    })
  }

  const handleSocksHostChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const nextConfig = {
      ...currentConfig,
      socksHost: event.target.value,
    }
    setDraftConfig(nextConfig)
  }

  const handlePersistSocksHost = () => {
    if (!currentConfig.enabled || !draftConfig) {
      return
    }

    void saveTorConfig(currentConfig)
  }

  const handleSocksPortChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const port = parseInt(event.target.value)
    if (!isNaN(port)) {
      setDraftConfig({
        ...currentConfig,
        socksPort: port,
      })
    }
  }

  const handlePersistSocksPort = () => {
    if (!currentConfig.enabled || !draftConfig) {
      return
    }

    void saveTorConfig(currentConfig)
  }

  const handleControlPortChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const port = Number.parseInt(event.target.value, 10)
    if (!Number.isNaN(port)) {
      setDraftConfig({
        ...currentConfig,
        controlPort: port,
      })
    }
  }

  const handlePersistControlPort = () => {
    if (!currentConfig.enabled || !draftConfig) {
      return
    }

    void saveTorConfig(currentConfig)
  }

  const handleUseBridgesChange = (checked: boolean) => {
    const nextConfig = {
      ...currentConfig,
      useBridges: checked,
    }

    setDraftConfig(nextConfig)
    void saveTorConfig(nextConfig)
  }

  const handleBridgesChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
    setDraftConfig({
      ...currentConfig,
      bridges: parseBridgeList(event.target.value),
    })
  }

  const handlePersistBridges = () => {
    if (!currentConfig.enabled || !draftConfig) {
      return
    }

    void saveTorConfig(currentConfig)
  }

  const handleCheckConnection = async () => {
    setTesting(true)

    try {
      const nextStatus = await testTorConnection()
      queryClient.setQueryData(torStatusQueryKey, nextStatus)
    } catch (error) {
      showNotice.error(error)
    } finally {
      setTesting(false)
    }
  }

  const handleCopySocksUrl = () => {
    navigator.clipboard.writeText(buildTorSocksUrl(currentConfig.socksHost, currentConfig.socksPort))
  }

  if (!verge) {
    return <div className="p-2 text-sm text-gray-500 dark:text-gray-400">加载中...</div>
  }

  return (
    <div>
      <div className="mb-2 flex items-center">
        <TorIcon className="mr-1 h-5 w-5" />
        <h6 className="flex-grow text-lg font-bold">
          Tor 代理
        </h6>
        <Switch checked={currentConfig.enabled} onCheckedChange={handleEnableChange} disabled={saving} />
      </div>

      <Alert severity="warning" className="mb-2">
        Tor 会显著降低网络速度（通常 &lt; 1 Mbps），仅在需要最强隐私保护时使用
      </Alert>

      {currentConfig.enabled && (
        <>
          <div className="mb-3">
            <div className="mb-1.5 text-sm text-gray-500 dark:text-gray-400">
              SOCKS5 代理配置
            </div>

            <div className="space-y-2">
              <TextField
                label="SOCKS5 主机"
                multiline={false}
                value={currentConfig.socksHost}
                onChange={handleSocksHostChange}
                onBlur={handlePersistSocksHost}
                fullWidth
                disabled={saving}
              />

              <TextField
                label="SOCKS5 端口"
                multiline={false}
                type="number"
                value={currentConfig.socksPort}
                onChange={handleSocksPortChange}
                onBlur={handlePersistSocksPort}
                fullWidth
                disabled={saving}
              />

              <TextField
                label="控制端口"
                multiline={false}
                type="number"
                value={currentConfig.controlPort}
                onChange={handleControlPortChange}
                onBlur={handlePersistControlPort}
                fullWidth
                disabled={saving}
              />

              <div className="flex items-center gap-1">
                <TextField
                  label="SOCKS5 代理地址"
                  multiline={false}
                  value={buildTorSocksUrl(currentConfig.socksHost, currentConfig.socksPort)}
                  fullWidth
                  readOnly
                />
                <IconButton onClick={handleCopySocksUrl} size="small">
                  <CopyIcon className="h-4 w-4" />
                </IconButton>
              </div>
            </div>
          </div>

          <div className="mb-3">
            <div className="mb-1.5 text-sm text-gray-500 dark:text-gray-400">
              网桥配置
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="text-sm">启用网桥</div>
                <Switch checked={currentConfig.useBridges} onCheckedChange={handleUseBridgesChange} disabled={saving} />
              </div>

              {currentConfig.useBridges && (
                <TextField
                  label="网桥列表"
                  helperText="每行一个网桥"
                  multiline
                  rows={4}
                  value={currentConfig.bridges.join('\n')}
                  onChange={handleBridgesChange}
                  onBlur={handlePersistBridges}
                  fullWidth
                  disabled={saving}
                />
              )}
            </div>
          </div>

          <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

          <div className="mb-3">
            <div className="mb-1.5 text-sm text-gray-500 dark:text-gray-400">
              连接状态
            </div>

            <div className="space-y-1.5">
              <div className="flex items-center gap-1">
                <div className="text-sm">Tor 状态:</div>
                {(status?.enabled ?? currentConfig.enabled) ? (
                  <Chip
                    icon={<CheckIcon className="h-3 w-3" />}
                    label="已启用"
                    color="success"
                    size="small"
                  />
                ) : (
                  <Chip icon={<ErrorIcon className="h-3 w-3" />} label="未启用" size="small" />
                )}
              </div>

              <div className="flex items-center gap-1">
                <div className="text-sm">连接状态:</div>
                {status?.connected ? (
                  <Chip
                    icon={<CheckIcon className="h-3 w-3" />}
                    label="已连接"
                    color="success"
                    size="small"
                  />
                ) : statusLoading || statusFetching || testing ? (
                  <Chip label="检测中" color="info" size="small" />
                ) : (
                  <Chip icon={<ErrorIcon className="h-3 w-3" />} label="未连接" color="error" size="small" />
                )}
              </div>

              {status?.assessment && (
                <div className="flex items-center gap-1">
                  <div className="text-sm">结果评估:</div>
                  <Chip
                    label={getAssessmentLabel(status.assessment)}
                    color={getAssessmentColor(status.assessment)}
                    size="small"
                  />
                </div>
              )}

              {status?.confidence && (
                <div className="flex items-center gap-1">
                  <div className="text-sm">结果置信度:</div>
                  <Chip label={getConfidenceLabel(status.confidence)} color="info" size="small" />
                </div>
              )}

              {status?.current_ip && (
                <div className="flex items-center gap-1 text-sm">
                  <div>出口 IP:</div>
                  <span className="uds-mono">{status.current_ip}</span>
                </div>
              )}

              {status?.exit_node && (
                <div className="flex items-center gap-1 text-sm">
                  <div>出口节点:</div>
                  <span>{status.exit_node}</span>
                </div>
              )}

              {status?.observation_path && (
                <div className="flex items-center gap-1 text-sm">
                  <div>观测路径:</div>
                  <span className="uds-mono">{status.observation_path}</span>
                </div>
              )}

              {status?.observation_source && (
                <div className="flex items-center gap-1 text-sm">
                  <div>观测源:</div>
                  <span className="uds-mono">{status.observation_source}</span>
                </div>
              )}

              {status?.runtime_risk_detected && status.runtime_risk_type.length > 0 ? (
                <Alert severity="warning" className="text-xs">
                  {status.runtime_risk_type.map(formatRuntimeRiskLabel).join('；')}
                </Alert>
              ) : null}

              {status?.observation_incomplete ? (
                <Alert severity="info" className="text-xs">
                  当前 Tor 观测不完整，结果仅反映已完成的 SOCKS5H 出口探测。
                </Alert>
              ) : null}

              {status?.warnings.length ? (
                <Alert severity="warning" className="text-xs">
                  {status.warnings.join('；')}
                </Alert>
              ) : null}

              {status?.error ? (
                <Alert severity="error" className="text-xs">
                  {status.error}
                </Alert>
              ) : null}

              <Button
                variant="outlined"
                size="small"
                onClick={handleCheckConnection}
                fullWidth
                loading={statusFetching || testing}
                disabled={saving || testing}
              >
                检查连接
              </Button>
            </div>
          </div>
        </>
      )}

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div>
        <Button
          onClick={() => setShowInstructions(!showInstructions)}
          endIcon={
            <ExpandMoreIcon
              className={cn(
                'h-4 w-4 transition-transform duration-300',
                showInstructions ? 'rotate-180' : 'rotate-0',
              )}
            />
          }
          fullWidth
        >
          使用说明
        </Button>

        <Collapse in={showInstructions}>
          <div className="mt-2">
            <div className="mb-1 text-sm font-medium">
              {torUsageInstructions.title}
            </div>

            <List dense>
              {torUsageInstructions.steps.map((step) => (
                <ListItem key={step}>
                  <ListItemText primary={step} />
                </ListItem>
              ))}
            </List>

            <div className="mb-1 mt-2 text-sm font-medium">
              注意事项
            </div>

            <List dense>
              {torUsageInstructions.notes.map((note) => (
                <ListItem key={note}>
                  <ListItemText primary={note} />
                </ListItem>
              ))}
            </List>
          </div>
        </Collapse>
      </div>
    </div>
  )
}
