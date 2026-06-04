import { useLockFn } from 'ahooks'
import { Globe, Route } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { closeAllConnections } from 'tauri-plugin-mihomo-api'

import { useRuntimeConfig } from '@/hooks/data/use-clash'
import { useVerge } from '@/hooks/system'
import {
  useAppRefreshers,
  useClashConfigData,
  useCoreDataStatus,
} from '@/providers/app-data-context'
import { resolveClashMode } from '@/services/clash-mode'
import { patchClashMode } from '@/services/cmds'
import { queryClient } from '@/services/query-client'
import { cn } from '@/utils/cn'

const HOME_PROXY_CHAIN_MODES = ['rule', 'global'] as const

type HomeProxyChainMode = (typeof HOME_PROXY_CHAIN_MODES)[number]

const MODE_META: Record<
  HomeProxyChainMode,
  { label: string; description: string }
> = {
  rule: {
    label: '应用规则',
    description: '按订阅规则分流',
  },
  global: {
    label: '不应用规则',
    description: '代理链路不套规则',
  },
}

export const ClashModeCard = () => {
  const { t } = useTranslation()
  const { verge } = useVerge()
  const { clashConfig } = useClashConfigData()
  const { isCoreDataPending } = useCoreDataStatus()
  const { refreshClashConfig } = useAppRefreshers()
  const { data: runtimeConfig } = useRuntimeConfig()
  const [optimisticMode, setOptimisticMode] = useState<
    HomeProxyChainMode | undefined
  >()

  // 直连是顶部全局出口；首页卡片只展示代理链路内部的规则状态。
  const currentModeKey = resolveClashMode(clashConfig?.mode, runtimeConfig?.mode)
  const displayMode =
    optimisticMode ?? (currentModeKey === 'direct' ? 'rule' : currentModeKey)

  const modeDescription = useMemo(() => {
    if (displayMode) {
      return MODE_META[displayMode].description
    }
    if (isCoreDataPending) {
      return '\u00A0'
    }
    return t('home.components.clashMode.errors.communication')
  }, [displayMode, isCoreDataPending, t])

  // 模式图标映射
  const modeIcons = useMemo(
    () => ({
      rule: <Route className="h-4 w-4" />,
      global: <Globe className="h-4 w-4" />,
    }),
    [],
  )

  // 切换模式的处理函数
  const onChangeMode = useLockFn(async (mode: HomeProxyChainMode) => {
    if (mode === displayMode) return
    if (verge?.auto_close_connection) {
      closeAllConnections()
    }

    setOptimisticMode(mode)
    queryClient.setQueryData(['getClashConfig'], (old: any) =>
      old ? { ...old, mode } : old,
    )
    queryClient.setQueryData(['getRuntimeConfig'], (old: any) =>
      old ? { ...old, mode } : old,
    )

    try {
      await patchClashMode(mode)
    } catch (error) {
      console.error('Failed to change mode:', error)
    } finally {
      await Promise.all([
        refreshClashConfig(),
        queryClient.invalidateQueries({ queryKey: ['getRuntimeConfig'] }),
      ])
      setOptimisticMode(undefined)
    }
  })

  return (
    <div className="flex flex-col w-full mt-1">
      {/* 模式选择按钮组 - 工业滑块选择器 */}
      <div className="flex items-center justify-between p-1 h-10 bg-action-hover/[0.02] border border-solid border-divider rounded-3xl w-full">
        {HOME_PROXY_CHAIN_MODES.map((mode) => {
          const isActive = mode === displayMode
          return (
            <div
              key={mode}
              onClick={() => onChangeMode(mode)}
              className={cn(
                'cursor-pointer px-3 h-8 flex items-center justify-center gap-2',
                'rounded-[20px] border-none transition-all duration-[250ms] ease-[cubic-bezier(0.16,1,0.3,1)]',
                'flex-1 max-w-[160px]',
                isActive
                  ? 'bg-primary text-primary-contrast shadow-[0_2px_8px_-2px_rgba(var(--color-primary-rgb),0.3)]'
                  : 'bg-transparent text-text-secondary hover:text-text-primary hover:bg-action-hover/5 hover:scale-[1.02]',
                'active:scale-[0.98]'
              )}
            >
              {modeIcons[mode]}
              <span className="text-[11px] capitalize tracking-[0.02em] font-semibold">
                {MODE_META[mode].label}
              </span>
            </div>
          )
        })}
      </div>

      {/* 说明文本区域 - 微型 Badge 元数据排版 */}
      <div className="w-full mt-3 flex items-center gap-3 px-1">
        <div className="inline-flex items-center h-[18px] px-3 rounded-full bg-primary/8 text-primary text-[8px] font-sans font-semibold uppercase tracking-[0.1em] flex-shrink-0">
          {displayMode || 'INFO'}
        </div>
        <p className="text-[9px] font-semibold uppercase tracking-[0.15em] text-text-secondary opacity-60 break-words leading-tight">
          {modeDescription}
        </p>
      </div>
    </div>
  )
}
