/**
 * XDP 代理配置组件
 */

import { useEffect, useState } from 'react'

import {
  type XdpConfig,
  type XdpSupportInfo,
  xdpCheckSupport,
  xdpGetConfig,
  xdpGetInterfaces,
  xdpGetStatus,
  xdpStart,
  xdpStop,
  xdpUpdateConfig,
  xdpUpdateStats,
} from '@/services/xdp'
import { showNotice } from '@/services/notice-service'
import { useMultiConfigLoader, useConfigSaver } from '@/hooks'
import XdpConfigUI from './xdp-config-ui'

export default function XdpConfigComponent() {
  const [localConfig, setLocalConfig] = useState<XdpConfig>({
    enabled: false,
    interface: 'eth0',
    mode: 'Skb',
    enable_stats: true,
  })
  const [supportInfo, setSupportInfo] = useState<XdpSupportInfo | null>(null)
  const [interfaces, setInterfaces] = useState<string[]>([])
  const [loading, setLoading] = useState(false)

  // 使用通用 Hook 加载配置和状态
  const { data, reload } = useMultiConfigLoader({
    loaders: {
      config: xdpGetConfig,
      status: xdpGetStatus,
    },
    onSuccess: (result) => {
      setLocalConfig(result.config)
    },
  })

  // 使用通用 Hook 保存配置
  const { save, saving } = useConfigSaver({
    saveFn: xdpUpdateConfig,
    onSuccess: reload,
    successMessage: '配置已保存',
  })

  // 加载支持信息和网卡列表
  useEffect(() => {
    checkSupport()
    loadInterfaces()

    // 定期更新状态
    const interval = setInterval(() => {
      reload()
      if (data?.status?.running) {
        updateStats()
      }
    }, 2000)

    return () => clearInterval(interval)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data?.status?.running])

  const checkSupport = async () => {
    try {
      const info = await xdpCheckSupport()
      setSupportInfo(info)
    } catch (error) {
      console.error('检查支持失败:', error)
    }
  }

  const loadInterfaces = async () => {
    try {
      const ifaces = await xdpGetInterfaces()
      setInterfaces(ifaces)
    } catch (error) {
      console.error('加载网卡列表失败:', error)
    }
  }

  const updateStats = async () => {
    try {
      await xdpUpdateStats()
      await reload()
    } catch (error) {
      console.error('更新统计失败:', error)
    }
  }

  const handleSaveConfig = () => {
    save(localConfig)
  }

  const handleStart = async () => {
    try {
      setLoading(true)
      await xdpStart()
      await reload()
      showNotice('success', 'XDP 代理已启动')
    } catch (error: any) {
      showNotice('error', `启动失败: ${error.message || error}`)
    } finally {
      setLoading(false)
    }
  }

  const handleStop = async () => {
    try {
      setLoading(true)
      await xdpStop()
      await reload()
      showNotice('success', 'XDP 代理已停止')
    } catch (error: any) {
      showNotice('error', `停止失败: ${error.message || error}`)
    } finally {
      setLoading(false)
    }
  }

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`
    if (bytes < 1024 * 1024 * 1024)
      return `${(bytes / 1024 / 1024).toFixed(2)} MB`
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
  }

  const formatNumber = (num: number) => {
    return num.toLocaleString()
  }

  const status = data?.status

  return (
    <XdpConfigUI
      config={localConfig}
      status={status || null}
      supportInfo={supportInfo}
      interfaces={interfaces}
      saving={saving}
      loading={loading}
      onConfigChange={setLocalConfig}
      onSaveConfig={handleSaveConfig}
      onStart={handleStart}
      onStop={handleStop}
      formatBytes={formatBytes}
      formatNumber={formatNumber}
    />
  )
}
