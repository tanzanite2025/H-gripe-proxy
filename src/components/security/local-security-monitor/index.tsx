import { RefreshCw, Shield } from 'lucide-react'
import { useEffect, useState, type ChangeEvent } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { TextField } from '@/components/tailwind/TextField'
import {
  checkSecurityNow,
  configureFirewall,
  findAvailablePort,
  getLeakMonitorPort,
  getLocalSecurityConfig,
  getLocalSecurityStatus,
  isLeakMonitorRunning,
  removeFirewall,
  startLeakMonitor,
  stopLeakMonitor,
  updateLocalSecurityConfig,
  type LocalSecurityConfig,
  type LeakMonitorStatus,
} from '@/services/local-security'
import { showNotice } from '@/services/notice-service'

import { SectionCard, ToggleRow } from './shared'
import { StatusSummary } from './status-summary'

export function LocalSecurityMonitor() {
  const [config, setConfig] = useState<LocalSecurityConfig | null>(null)
  const [status, setStatus] = useState<LeakMonitorStatus | null>(null)
  const [loading, setLoading] = useState(false)
  const [port, setPort] = useState(10808)
  const [monitorRunning, setMonitorRunning] = useState(false)

  const loadData = async () => {
    try {
      const [nextConfig, nextStatus, running, monitorPort] = await Promise.all([
        getLocalSecurityConfig(),
        getLocalSecurityStatus(),
        isLeakMonitorRunning(),
        getLeakMonitorPort(),
      ])

      setConfig(nextConfig)
      setStatus(nextStatus)
      setMonitorRunning(running)
      setPort(monitorPort)
    } catch (error) {
      console.error('Failed to load local security data:', error)
      showNotice.error('加载本地安全数据失败。', error)
    }
  }

  useEffect(() => {
    void loadData()
  }, [])

  const persistConfig = async (nextConfig: LocalSecurityConfig) => {
    try {
      await updateLocalSecurityConfig(nextConfig)
      setConfig(nextConfig)
      showNotice.success('配置已更新。')
    } catch (error) {
      console.error('Failed to update local security config:', error)
      showNotice.error('更新本地安全配置失败。', error)
    }
  }

  const handleConfigChange = async (
    updates: Partial<LocalSecurityConfig>,
  ) => {
    if (!config) return
    await persistConfig({ ...config, ...updates })
  }

  const handleCheckNow = async () => {
    setLoading(true)
    try {
      const nextStatus = await checkSecurityNow(port)
      setStatus(nextStatus)
      showNotice.success('安全检查已完成。')
    } catch (error) {
      console.error('Failed to check local security:', error)
      showNotice.error('本地安全检查失败。', error)
    } finally {
      setLoading(false)
    }
  }

  const handleConfigureFirewall = async () => {
    setLoading(true)
    try {
      await configureFirewall(port)
      showNotice.success('防火墙规则已配置。')
      await loadData()
    } catch (error) {
      console.error('Failed to configure firewall:', error)
      showNotice.error('配置防火墙规则失败。', error)
    } finally {
      setLoading(false)
    }
  }

  const handleRemoveFirewall = async () => {
    setLoading(true)
    try {
      await removeFirewall(port)
      showNotice.success('防火墙规则已删除。')
      await loadData()
    } catch (error) {
      console.error('Failed to remove firewall rules:', error)
      showNotice.error('删除防火墙规则失败。', error)
    } finally {
      setLoading(false)
    }
  }

  const handleFindAvailablePort = async () => {
    setLoading(true)
    try {
      const nextPort = await findAvailablePort()
      setPort(nextPort)
      showNotice.success(`已找到可用端口 ${nextPort}。`)
    } catch (error) {
      console.error('Failed to find available port:', error)
      showNotice.error('查找可用端口失败。', error)
    } finally {
      setLoading(false)
    }
  }

  const handleStartMonitor = async () => {
    setLoading(true)
    try {
      await startLeakMonitor(port)
      setMonitorRunning(true)
      showNotice.success('泄漏监控已启动。')
      await loadData()
    } catch (error) {
      console.error('Failed to start leak monitor:', error)
      showNotice.error('启动泄漏监控失败。', error)
    } finally {
      setLoading(false)
    }
  }

  const handleStopMonitor = async () => {
    setLoading(true)
    try {
      await stopLeakMonitor()
      setMonitorRunning(false)
      showNotice.success('泄漏监控已停止。')
      await loadData()
    } catch (error) {
      console.error('Failed to stop leak monitor:', error)
      showNotice.error('停止泄漏监控失败。', error)
    } finally {
      setLoading(false)
    }
  }

  if (!config || !status) {
    return (
      <Card className="p-6">
        <div className="text-sm text-gray-500 dark:text-gray-400">加载中...</div>
      </Card>
    )
  }

  return (
    <Card className="p-6">
      <div className="flex items-start gap-3">
        <div className="rounded-full bg-primary/10 p-2 text-primary">
          <Shield className="h-5 w-5" />
        </div>
        <div className="flex-1">
          <h3 className="text-lg font-semibold text-text-primary">
            本地安全监控
          </h3>
          <p className="mt-1 text-sm text-text-secondary">
            围绕本地监听绑定、防火墙规则和泄漏监控做持续检查，尽量减少入口侧暴露。
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Chip
              size="small"
              color={config.autoFirewall ? 'success' : 'default'}
              label={config.autoFirewall ? '自动防火墙开启' : '自动防火墙关闭'}
            />
            <Chip
              size="small"
              color={config.leakMonitoring ? 'info' : 'default'}
              label={config.leakMonitoring ? '泄漏监控开启' : '泄漏监控关闭'}
            />
            <Chip
              size="small"
              color="default"
              label={`绑定地址 ${config.bindAddress}`}
            />
          </div>
        </div>
      </div>

      <div className="mt-6 space-y-4">
        <Alert severity="info" className="text-sm">
          这层保护主要关注本机入口暴露面。它和出口侧的节点隐匿是互补关系，长期建议一起使用。
        </Alert>

        <StatusSummary status={status} monitorRunning={monitorRunning} />

        <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
          <SectionCard
            title="保护选项"
            description="控制本地安全策略是否自动参与防护、监控和冲突处理。"
          >
            <ToggleRow
              title="自动配置防火墙"
              description="在本地监听端口变化时自动处理防火墙规则。"
              checked={config.autoFirewall}
              disabled={loading}
              onCheckedChange={(checked) =>
                void handleConfigChange({ autoFirewall: checked })
              }
            />
            <ToggleRow
              title="启用泄漏监控"
              description="持续监视本地监听和外部访问暴露，适合长期开着。"
              checked={config.leakMonitoring}
              disabled={loading}
              onCheckedChange={(checked) =>
                void handleConfigChange({ leakMonitoring: checked })
              }
            />
            <ToggleRow
              title="端口冲突自动切换"
              description="当目标端口被占用时，允许系统自动寻找替代端口。"
              checked={config.autoSwitchOnConflict}
              disabled={loading}
              onCheckedChange={(checked) =>
                void handleConfigChange({ autoSwitchOnConflict: checked })
              }
            />
            <ToggleRow
              title="进程隐匿"
              description="让本地入口侧与隐匿策略联动，降低桌面环境特征暴露。"
              checked={config.processStealth}
              disabled={loading}
              onCheckedChange={(checked) =>
                void handleConfigChange({ processStealth: checked })
              }
            />
            <ToggleRow
              title="端口随机化"
              description="允许在设定范围内切换监听端口，减少长期固定端口特征。"
              checked={config.portRandomization}
              disabled={loading}
              onCheckedChange={(checked) =>
                void handleConfigChange({ portRandomization: checked })
              }
            />
            <div className="text-xs text-text-secondary">
              当前端口范围：{config.portRange[0]} - {config.portRange[1]}
            </div>
            <TextField
              label="监控间隔（秒）"
              type="number"
              value={String(config.monitorInterval)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                void handleConfigChange({
                  monitorInterval:
                    Number.parseInt(event.target.value, 10) ||
                    config.monitorInterval,
                })
              }
              fullWidth
              size="small"
            />
          </SectionCard>

          <SectionCard
            title="防火墙管理"
            description="围绕当前监听端口配置本地防火墙规则，阻断非预期外部访问。"
          >
            <TextField
              label="监听端口"
              type="number"
              value={String(port)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                setPort(Number.parseInt(event.target.value, 10) || port)
              }
              size="small"
              fullWidth
            />
            <div className="flex flex-wrap gap-2">
              <Button
                variant="contained"
                size="small"
                onClick={() => void handleConfigureFirewall()}
                disabled={loading}
              >
                配置防火墙
              </Button>
              <Button
                variant="outlined"
                size="small"
                onClick={() => void handleRemoveFirewall()}
                disabled={loading}
              >
                删除规则
              </Button>
              <Button
                variant="outlined"
                size="small"
                onClick={() => void handleFindAvailablePort()}
                disabled={loading}
              >
                查找可用端口
              </Button>
            </div>
            <div className="text-xs text-text-secondary">
              建议只允许本地回环访问，避免外部网段直接接触入口监听端口。
            </div>
          </SectionCard>

          <SectionCard
            title="监控动作"
            description="手动触发检查或启动长期监控，便于快速复查当前入口侧暴露状态。"
          >
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outlined"
                startIcon={<RefreshCw className="h-4 w-4" />}
                onClick={() => void handleCheckNow()}
                disabled={loading}
              >
                立即检查
              </Button>
              {monitorRunning ? (
                <Button
                  variant="contained"
                  color="error"
                  onClick={() => void handleStopMonitor()}
                  disabled={loading}
                >
                  停止监控
                </Button>
              ) : (
                <Button
                  variant="contained"
                  color="success"
                  onClick={() => void handleStartMonitor()}
                  disabled={loading}
                >
                  启动监控
                </Button>
              )}
            </div>

            {status.lastCheckTime > 0 ? (
              <div className="text-xs text-text-secondary">
                最后检查：{new Date(status.lastCheckTime * 1000).toLocaleString()}
              </div>
            ) : (
              <div className="text-xs text-text-secondary">
                暂无最近检查记录。
              </div>
            )}
          </SectionCard>
        </div>
      </div>
    </Card>
  )
}
