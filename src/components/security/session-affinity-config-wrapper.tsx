/**
 * 会话绑定配置容器组件（安全页用）
 *
 * 负责通过 AdvancedConfig 加载/保存 session_affinity 配置，
 * 并复用受控的 SessionAffinityConfig 作为实际表单 UI。
 */

import { useEffect, useState } from 'react'

import { useConfigLoader, useConfigSaver } from '@/hooks'
import { type AdvancedConfig, getAdvancedConfig, saveAdvancedConfig } from '@/services/coordinator'
import type { SessionAffinityConfig as SessionAffinityConfigModel } from '@/services/session-affinity'

import { SessionAffinityConfig } from './session-affinity-config'

export default function SessionAffinityConfigComponent() {
  const { data: advancedConfig, loading, reload } = useConfigLoader<AdvancedConfig>({
    loadFn: getAdvancedConfig,
  })

  const { save, saving } = useConfigSaver<AdvancedConfig>({
    saveFn: saveAdvancedConfig,
    onSuccess: reload,
    successMessage: '会话绑定配置已保存',
  })

  const [localConfig, setLocalConfig] = useState<SessionAffinityConfigModel | null>(null)

  useEffect(() => {
    if (advancedConfig) {
      setLocalConfig(advancedConfig.session_affinity)
    }
  }, [advancedConfig])

  const handleSave = () => {
    if (!advancedConfig || !localConfig) return

    const updatedConfig: AdvancedConfig = {
      ...advancedConfig,
      session_affinity: localConfig,
    }

    void save(updatedConfig)
  }

  if (loading || !advancedConfig || !localConfig) {
    return (
      <div className="p-6">
        <p>加载中...</p>
      </div>
    )
  }

  return (
    <div className="space-y-4 p-6">
      <SessionAffinityConfig config={localConfig} onChange={setLocalConfig} />

      <div className="flex justify-end">
        <button
          type="button"
          onClick={handleSave}
          disabled={saving}
          className="inline-flex items-center px-4 py-2 bg-primary text-primary-foreground rounded-md text-sm font-medium hover:bg-primary/90 disabled:opacity-50"
        >
          {saving ? '保存中...' : '保存配置'}
        </button>
      </div>
    </div>
  )
}
