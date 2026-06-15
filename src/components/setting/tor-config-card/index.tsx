import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Copy as CopyIcon } from 'lucide-react'
import { useMemo, useState, type ChangeEvent } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { IconButton } from '@/components/tailwind/IconButton'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import { useVerge } from '@/hooks/system'
import {
  getTorStatus,
  testTorConnection,
  type TorRuntimeStatus,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import { buildTorRuntimeViewModel } from '../tor-runtime-view-model'

import {
  buildTorSocksUrl,
  DEFAULT_TOR_CONFIG,
  parseBridgeList,
  type TorDraftConfig,
} from './constants'
import { InstructionsDialog } from './instructions-dialog'
import { RuntimeStatusPanel } from './runtime-status-panel'

export const TorConfigCard = () => {
  const { verge, patchVerge } = useVerge()
  const queryClient = useQueryClient()
  const [draftConfig, setDraftConfig] = useState<TorDraftConfig | null>(null)
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [instructionsDialogOpen, setInstructionsDialogOpen] = useState(false)

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
    enabled: !!verge && persistedConfig.enabled,
    refetchOnWindowFocus: false,
    retry: false,
  })

  const runtimeView = buildTorRuntimeViewModel(
    status,
    currentConfig.enabled,
    statusLoading || statusFetching || testing,
  )

  const saveTorConfig = async (nextConfig: TorDraftConfig) => {
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
      showNotice.error('保存 Tor 配置失败。', error)
    } finally {
      setSaving(false)
    }
  }

  const updateDraft = (patch: Partial<TorDraftConfig>) => {
    setDraftConfig({
      ...currentConfig,
      ...patch,
    })
  }

  const persistDraft = () => {
    if (!currentConfig.enabled || !draftConfig) {
      return
    }

    void saveTorConfig(currentConfig)
  }

  const handleEnableChange = (checked: boolean) => {
    void saveTorConfig({
      ...currentConfig,
      enabled: checked,
    })
  }

  const handleNumericDraftChange =
    (key: 'socksPort' | 'controlPort') =>
    (event: ChangeEvent<HTMLInputElement>) => {
      const port = Number.parseInt(event.target.value, 10)
      if (!Number.isNaN(port)) {
        updateDraft({ [key]: port } as Partial<TorDraftConfig>)
      }
    }

  const handleUseBridgesChange = (checked: boolean) => {
    const nextConfig = {
      ...currentConfig,
      useBridges: checked,
    }

    setDraftConfig(nextConfig)
    void saveTorConfig(nextConfig)
  }

  const handleCheckConnection = async () => {
    setTesting(true)

    try {
      const nextStatus = await testTorConnection()
      queryClient.setQueryData<TorRuntimeStatus>(torStatusQueryKey, nextStatus)
    } catch (error) {
      showNotice.error('Tor 连接检测失败。', error)
    } finally {
      setTesting(false)
    }
  }

  const handleCopySocksUrl = async () => {
    try {
      await navigator.clipboard.writeText(
        buildTorSocksUrl(currentConfig.socksHost, currentConfig.socksPort),
      )
      showNotice.success('已复制 SOCKS5 地址。')
    } catch (error) {
      showNotice.error('复制 SOCKS5 地址失败。', error)
    }
  }

  if (!verge) {
    return (
      <div className="p-2 text-sm text-gray-500 dark:text-gray-400">
        加载中...
      </div>
    )
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-y-auto pr-1">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <Switch
              checked={currentConfig.enabled}
              onCheckedChange={handleEnableChange}
              disabled={saving}
            />
            <Chip
              size="small"
              color={currentConfig.enabled ? 'success' : 'default'}
              label={currentConfig.enabled ? 'Tor 已启用' : 'Tor 未启用'}
            />
            {currentConfig.useBridges ? (
              <Chip
                size="small"
                color="info"
                label={`桥接 ${currentConfig.bridges.length} 条`}
              />
            ) : null}
          </div>

          <div className="mt-2 text-xs text-gray-500 dark:text-gray-400">
            Tor 适合高隐私需求场景，但会显著降低速度和稳定性。
          </div>
        </div>

        <Button size="small" onClick={() => setInstructionsDialogOpen(true)}>
          使用说明
        </Button>
      </div>

      <Alert severity="warning" className="text-xs">
        Tor 会显著降低网络速度，通常不适合常规日用流量；更适合临时高隐私场景或特定访问需求。
      </Alert>

      {currentConfig.enabled ? (
        <>
          <section className="space-y-2">
            <div className="text-sm text-gray-500 dark:text-gray-400">
              SOCKS5 代理配置
            </div>

            <div className="space-y-2">
              <TextField
                label="SOCKS5 主机"
                value={currentConfig.socksHost}
                onChange={(event: ChangeEvent<HTMLInputElement>) =>
                  updateDraft({ socksHost: event.target.value })
                }
                onBlur={persistDraft}
                fullWidth
                disabled={saving}
              />

              <TextField
                label="SOCKS5 端口"
                type="number"
                value={String(currentConfig.socksPort)}
                onChange={handleNumericDraftChange('socksPort')}
                onBlur={persistDraft}
                fullWidth
                disabled={saving}
              />

              <TextField
                label="控制端口"
                type="number"
                value={String(currentConfig.controlPort)}
                onChange={handleNumericDraftChange('controlPort')}
                onBlur={persistDraft}
                fullWidth
                disabled={saving}
              />

              <div className="flex items-end gap-1">
                <TextField
                  label="SOCKS5 地址"
                  value={buildTorSocksUrl(
                    currentConfig.socksHost,
                    currentConfig.socksPort,
                  )}
                  fullWidth
                  readOnly
                />
                <IconButton onClick={() => void handleCopySocksUrl()} size="small">
                  <CopyIcon className="h-4 w-4" />
                </IconButton>
              </div>
            </div>
          </section>

          <section className="space-y-2">
            <div className="text-sm text-gray-500 dark:text-gray-400">
              桥接配置
            </div>

            <div className="flex items-center justify-between">
              <div className="text-sm">启用桥接</div>
              <Switch
                checked={currentConfig.useBridges}
                onCheckedChange={handleUseBridgesChange}
                disabled={saving}
              />
            </div>

            {currentConfig.useBridges ? (
              <TextField
                label="桥接列表"
                helperText="每行一条 bridge；保存后会写入当前 Tor 配置。"
                multiline
                rows={4}
                value={currentConfig.bridges.join('\n')}
                onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                  updateDraft({ bridges: parseBridgeList(event.target.value) })
                }
                onBlur={persistDraft}
                fullWidth
                disabled={saving}
              />
            ) : null}
          </section>

          <div className="border-t border-gray-200 dark:border-gray-700" />

          <RuntimeStatusPanel
            runtimeView={runtimeView}
            status={status}
            checking={statusFetching || testing}
            disabled={saving || testing}
            onCheckConnection={() => void handleCheckConnection()}
          />
        </>
      ) : null}

      <InstructionsDialog
        open={instructionsDialogOpen}
        onClose={() => setInstructionsDialogOpen(false)}
      />
    </div>
  )
}
