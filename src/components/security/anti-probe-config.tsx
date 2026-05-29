/**
 * 反主动探测配置组件
 */

import { useEffect, useState } from 'react'

import { useConfigLoader, useConfigSaver } from '@/hooks'
import { antiProbeCleanup, antiProbeGenerateToken } from '@/services/anti-probe'
import {
  type AdvancedConfig,
  type AntiProbeConfig,
  getAdvancedConfig,
  saveAdvancedConfig,
} from '@/services/coordinator'
import { showNotice } from '@/services/notice-service'

import AntiProbeConfigUI from './anti-probe-config-ui'

export default function AntiProbeConfigComponent() {
  const [newIp, setNewIp] = useState('')
  const [token, setToken] = useState('')

  // 使用通用 Hook 加载配置（AdvancedConfig）
  const { data: advancedConfig, loading, reload } = useConfigLoader<AdvancedConfig>({
    loadFn: getAdvancedConfig,
  })

  // 使用通用 Hook 保存配置（AdvancedConfig）
  const { save, saving } = useConfigSaver<AdvancedConfig>({
    saveFn: saveAdvancedConfig,
    onSuccess: reload,
    successMessage: '配置已保存',
  })

  // 本地 AntiProbe 配置状态（用于编辑）
  const [localConfig, setLocalConfig] = useState<AntiProbeConfig>({
    enabled: false,
    secret_key: '',
    time_window: 300,
    whitelist: [],
    strict_mode: false,
  })

  // 当 AdvancedConfig 加载完成时，更新本地 AntiProbe 配置
  useEffect(() => {
    if (advancedConfig) {
      setLocalConfig(advancedConfig.security.anti_probe)
    }
  }, [advancedConfig])

  // 保存配置：只更新 AdvancedConfig.security.anti_probe
  const handleSave = () => {
    if (!advancedConfig) return

    const updatedConfig: AdvancedConfig = {
      ...advancedConfig,
      security: {
        ...advancedConfig.security,
        anti_probe: localConfig,
      },
    }

    void save(updatedConfig)
  }

  // 生成新密钥
  const handleGenerateKey = () => {
    const newKey = Array.from({ length: 32 }, () =>
      Math.floor(Math.random() * 256)
        .toString(16)
        .padStart(2, '0'),
    ).join('')
    setLocalConfig({ ...localConfig, secret_key: newKey })
  }

  // 生成握手暗号
  const handleGenerateToken = async () => {
    try {
      const newToken = await antiProbeGenerateToken()
      setToken(newToken)
      showNotice('success', '握手暗号已生成')
    } catch (error: any) {
      showNotice('error', `生成暗号失败: ${error.message || error}`)
    }
  }

  // 复制暗号
  const handleCopyToken = () => {
    navigator.clipboard.writeText(token)
    showNotice('success', '已复制到剪贴板')
  }

  // 添加白名单 IP
  const handleAddIp = () => {
    if (!newIp.trim()) return

    // 简单的 IP 格式验证
    const ipRegex =
      /^(\d{1,3}\.){3}\d{1,3}$|^([0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}$/
    if (!ipRegex.test(newIp)) {
      showNotice('error', '无效的 IP 地址格式')
      return
    }

    if (localConfig.whitelist.includes(newIp)) {
      showNotice('error', 'IP 已存在于白名单')
      return
    }

    setLocalConfig({
      ...localConfig,
      whitelist: [...localConfig.whitelist, newIp],
    })
    setNewIp('')
  }

  // 删除白名单 IP
  const handleRemoveIp = (ip: string) => {
    setLocalConfig({
      ...localConfig,
      whitelist: localConfig.whitelist.filter((i: string) => i !== ip),
    })
  }

  // 清理过期缓存
  const handleCleanup = async () => {
    try {
      await antiProbeCleanup()
      showNotice('success', '已清理过期缓存')
    } catch (error: any) {
      showNotice('error', `清理失败: ${error.message || error}`)
    }
  }

  if (loading || !advancedConfig) {
    return (
      <div className="p-6">
        <p>加载中...</p>
      </div>
    )
  }

  return (
    <AntiProbeConfigUI
      config={localConfig}
      token={token}
      newIp={newIp}
      saving={saving}
      onConfigChange={setLocalConfig}
      onTokenGenerate={handleGenerateToken}
      onTokenCopy={handleCopyToken}
      onNewIpChange={setNewIp}
      onAddIp={handleAddIp}
      onRemoveIp={handleRemoveIp}
      onGenerateKey={handleGenerateKey}
      onSave={handleSave}
      onCleanup={handleCleanup}
    />
  )
}