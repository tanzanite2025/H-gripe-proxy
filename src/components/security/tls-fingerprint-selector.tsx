/**
 * TLS 指纹选择器组件
 */

import { CheckCircle, Fingerprint, Info } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Button } from '@/components/tailwind'
import { useConfigLoader, useConfigSaver } from '@/hooks'
import {
  type AdvancedConfig,
  getAdvancedConfig,
  saveAdvancedConfig,
} from '@/services/coordinator'
import { showNotice } from '@/services/notice-service'
import { type TlsFingerprint, tlsFingerprintGetAll } from '@/services/tls-fingerprint'
import { cn } from '@/utils/cn'

export default function TlsFingerprintSelector() {
  const [fingerprints, setFingerprints] = useState<TlsFingerprint[]>([])
  const [selectedName, setSelectedName] = useState<string | null>(null)
  const [fingerprintsLoading, setFingerprintsLoading] = useState(false)

  const {
    data: advancedConfig,
    loading: configLoading,
    reload,
  } = useConfigLoader<AdvancedConfig>({
    loadFn: getAdvancedConfig,
  })

  const { save, saving } = useConfigSaver<AdvancedConfig>({
    saveFn: saveAdvancedConfig,
    onSuccess: reload,
    successMessage: 'TLS 指纹配置已保存',
  })

  // 加载指纹列表
  useEffect(() => {
    void loadFingerprints()
  }, [])

  const loadFingerprints = async () => {
    try {
      setFingerprintsLoading(true)
      const fps = await tlsFingerprintGetAll()
      setFingerprints(fps)
    } catch (error) {
      showNotice.error(`加载指纹列表失败: ${error}`)
    } finally {
      setFingerprintsLoading(false)
    }
  }
  
  // 当高级配置加载完成时，同步当前选择的指纹名称
  useEffect(() => {
    if (advancedConfig) {
      setSelectedName(advancedConfig.security.tls_fingerprint)
    }
  }, [advancedConfig])

  // 选择指纹
  const handleSelectFingerprint = async (name: string) => {
    if (!advancedConfig) return

    setSelectedName(name)

    const updated: AdvancedConfig = {
      ...advancedConfig,
      security: {
        ...advancedConfig.security,
        tls_fingerprint: name,
      },
    }

    void save(updated)
  }

  // 清除指纹
  const handleClearFingerprint = async () => {
    if (!advancedConfig) return

    setSelectedName(null)

    const updated: AdvancedConfig = {
      ...advancedConfig,
      security: {
        ...advancedConfig.security,
        tls_fingerprint: null,
      },
    }

    void save(updated)
  }

  // 获取指纹图标
  const getFingerprintIcon = (name: string) => {
    if (name.includes('Chrome')) return '🌐'
    if (name.includes('Firefox')) return '🦊'
    if (name.includes('Safari')) return '🧭'
    if (name.includes('Genshin')) return '🎮'
    return '🔒'
  }

  const isBusy = configLoading || fingerprintsLoading || saving

  const currentFingerprint =
    fingerprints.find((fp) => fp.name === selectedName) ?? null

  if (configLoading || !advancedConfig || fingerprintsLoading) {
    return (
      <div className="p-6">
        <p>加载中...</p>
      </div>
    )
  }

  return (
    <div className="p-6">
      <div className="space-y-6">
        {/* 标题 */}
        <div className="flex items-center gap-2">
          <Fingerprint className="w-5 h-5 text-primary" />
          <h2 className="text-xl font-semibold">TLS 指纹伪装（Parrot Mode）</h2>
        </div>

        {/* 说明 */}
        <div className="p-4 bg-blue-500 text-white rounded-lg">
          <div className="flex items-start gap-2">
            <Info className="w-5 h-5 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-sm">ALPN 真实指纹复刻</p>
              <p className="text-xs opacity-90 mt-1">
                100% 复刻真实浏览器/应用的 TLS 指纹（JA3/JA4），让翻墙流量在统计学上和普通人刷网页、打游戏毫无二致。
              </p>
            </div>
          </div>
        </div>

        {/* 当前指纹（基于配置中的 tls_fingerprint 名称） */}
        {currentFingerprint && (
          <div className="p-4 bg-green-500 text-white rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <CheckCircle className="w-5 h-5" />
              <span className="text-sm font-semibold">当前使用指纹</span>
            </div>
            <h3 className="text-2xl font-bold mb-1">
              {getFingerprintIcon(currentFingerprint.name)}{' '}
              {currentFingerprint.name}
            </h3>
            <p className="text-xs opacity-90 mb-3">
              {currentFingerprint.description}
            </p>
            <Button
              variant="outline"
              size="sm"
              onClick={handleClearFingerprint}
              disabled={isBusy}
              className="border-white text-white hover:bg-white/10"
            >
              清除伪装
            </Button>
          </div>
        )}

        {/* 指纹列表 */}
        <h3 className="text-sm font-semibold">选择 TLS 指纹</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {fingerprints.map((fp) => {
            const isActive = selectedName === fp.name

            return (
              <div
                key={fp.name}
                className={cn(
                  'p-4 rounded-lg border-2 cursor-pointer transition-all duration-200',
                  'hover:border-primary hover:-translate-y-0.5 hover:shadow-lg',
                  isActive
                    ? 'border-primary bg-primary/5'
                    : 'border-divider bg-card'
                )}
                onClick={() => !isBusy && handleSelectFingerprint(fp.name)}
              >
                <div className="flex items-center gap-2 mb-2">
                  <span className="text-3xl">{getFingerprintIcon(fp.name)}</span>
                  <div className="flex-1">
                    <h4 className="font-semibold text-sm">{fp.name}</h4>
                    {isActive && (
                      <span className="inline-block px-2 py-0.5 text-xs bg-primary text-primary-foreground rounded-full mt-1">
                        使用中
                      </span>
                    )}
                  </div>
                </div>

                <p className="text-xs text-muted-foreground mb-3">
                  {fp.description}
                </p>

                <div className="space-y-2">
                  <div className="flex gap-2 flex-wrap">
                    <span className="px-2 py-1 text-xs bg-secondary rounded">
                      {fp.tls_version}
                    </span>
                    <span className="px-2 py-1 text-xs bg-secondary rounded">
                      {fp.cipher_suites.length} 密码套件
                    </span>
                  </div>

                  <div className="flex gap-2 flex-wrap">
                    {fp.alpn_protocols.map((alpn) => (
                      <span
                        key={alpn}
                        className="px-2 py-1 text-xs border border-border rounded"
                      >
                        {alpn}
                      </span>
                    ))}
                  </div>

                  <p className="text-[0.7rem] text-muted-foreground font-mono break-all">
                    JA3: {fp.ja3_fingerprint.substring(0, 40)}...
                  </p>
                </div>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}
