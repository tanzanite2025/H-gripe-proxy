import { useLockFn } from 'ahooks'
import { Globe, Route } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useRuntimeConfig } from '@/hooks/data/use-clash'
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
  const { clashConfig } = useClashConfigData()
  const { isCoreDataPending } = useCoreDataStatus()
  const { refreshClashConfig } = useAppRefreshers()
  const { data: runtimeConfig } = useRuntimeConfig()
  const [optimisticMode, setOptimisticMode] = useState<
    HomeProxyChainMode | undefined
  >()

  const currentModeKey = resolveClashMode(clashConfig?.mode, runtimeConfig?.mode)
  const displayMode = optimisticMode ?? currentModeKey

  const modeDescription = useMemo(() => {
    if (displayMode) {
      return MODE_META[displayMode].description
    }
    if (isCoreDataPending) {
      return '\u00A0'
    }
    return t('home.components.clashMode.errors.communication')
  }, [displayMode, isCoreDataPending, t])

  const modeIcons = useMemo(
    () => ({
      rule: <Route className="h-4 w-4" />,
      global: <Globe className="h-4 w-4" />,
    }),
    [],
  )

  const onChangeMode = useLockFn(async (mode: HomeProxyChainMode) => {
    if (mode === displayMode) return
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
    <div className="mt-1 flex w-full flex-col">
      <div className="flex h-10 w-full items-center justify-between rounded-3xl border border-solid border-divider bg-action-hover/[0.02] p-1">
        {HOME_PROXY_CHAIN_MODES.map((mode) => {
          const isActive = mode === displayMode
          return (
            <div
              key={mode}
              onClick={() => onChangeMode(mode)}
              className={cn(
                'flex-1 max-w-[160px]',
                'cursor-pointer px-3 h-8 flex items-center justify-center gap-2',
                'rounded-[20px] border-none transition-all duration-[250ms] ease-[cubic-bezier(0.16,1,0.3,1)]',
                isActive
                  ? 'bg-primary text-primary-contrast shadow-[0_2px_8px_-2px_rgba(var(--color-primary-rgb),0.3)]'
                  : 'bg-transparent text-text-secondary hover:text-text-primary hover:bg-action-hover/5 hover:scale-[1.02]',
                'active:scale-[0.98]',
              )}
            >
              {modeIcons[mode]}
              <span className="text-[11px] font-semibold capitalize tracking-[0.02em]">
                {MODE_META[mode].label}
              </span>
            </div>
          )
        })}
      </div>

      <div className="mt-3 flex w-full items-center gap-3 px-1">
        <div className="inline-flex h-[18px] shrink-0 items-center rounded-full bg-primary/8 px-3 font-sans text-[8px] font-semibold uppercase tracking-[0.1em] text-primary">
          {displayMode || 'INFO'}
        </div>
        <p className="break-words text-[9px] font-semibold uppercase leading-tight tracking-[0.15em] text-text-secondary opacity-60">
          {modeDescription}
        </p>
      </div>
    </div>
  )
}
