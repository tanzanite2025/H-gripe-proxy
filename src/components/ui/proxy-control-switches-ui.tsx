/**
 * 代理控制开关 UI 组件
 */

import { Pause, Play } from 'lucide-react'
import { useState, useRef } from 'react'

import { Switch } from '@/components/base'
import { cn } from '@/utils/cn'

interface SwitchRowProps {
  label: string
  active: boolean
  disabled?: boolean
  infoTitle: string
  onInfoClick?: () => void
  extraIcons?: React.ReactNode
  onToggle: (value: boolean) => Promise<void>
  onError?: (err: Error) => void
  highlight?: boolean
  compact?: boolean
}

/**
 * 抽取的子组件：统一的开关 UI
 * active = 真实状态OS/配置 乐观更新
 */
export const SwitchRow = ({
  label,
  active,
  disabled,
  infoTitle,
  onInfoClick,
  extraIcons,
  onToggle,
  onError,
  highlight,
  compact,
}: SwitchRowProps) => {
  const [checked, setChecked] = useState(active)
  const pendingRef = useRef(false)

  if (pendingRef.current) {
    if (active === checked) pendingRef.current = false
  } else if (checked !== active) {
    setChecked(active)
  }

  const handleChange = (_: React.ChangeEvent, value: boolean) => {
    pendingRef.current = true
    setChecked(value)
    onToggle(value)
      .catch((err: any) => {
        setChecked(active)
        onError?.(err)
      })
      .finally(() => {
        pendingRef.current = false
      })
  }

  if (compact) {
    return (
      <div
        className={cn(
          'flex items-center justify-between p-2 pr-4 rounded-xl transition-colors',
          highlight ? 'bg-green-500/10' : 'bg-transparent',
          disabled && 'opacity-60'
        )}
      >
        <div className="flex items-center">
          {active ? (
            <Play className="w-5 h-5 text-green-500 mr-2" />
          ) : (
            <Pause className="w-5 h-5 text-muted-foreground mr-2" />
          )}
          <h3 className="text-[15px] font-medium">{label}</h3>
          <button
            type="button"
            className="text-xs ml-2 px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
            onClick={onInfoClick}
          >
            设置
          </button>
          {extraIcons}
        </div>

        <Switch
          disabled={disabled}
          checked={checked}
          onChange={handleChange}
        />
      </div>
    )
  }

  return (
    <div className={cn('uds-settings-item', disabled && 'opacity-60')}>
      <div
        className={cn(
          'uds-settings-item__body',
          highlight && 'bg-green-500/10'
        )}
      >
        <div className="uds-settings-item__main">
          <div className="uds-settings-item__label-row">
            {active ? (
              <Play className="w-5 h-5 text-green-500" />
            ) : (
              <Pause className="w-5 h-5 text-muted-foreground" />
            )}
            <h3 className="uds-settings-item__label uds-card-title text-[15px] font-medium">
              {label}
            </h3>
            <button
              type="button"
              className="text-xs ml-2 px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
              onClick={onInfoClick}
            >
              设置
            </button>
            {extraIcons}
          </div>
        </div>

        <div className="uds-settings-item__control">
          <Switch
            disabled={disabled}
            checked={checked}
            onChange={handleChange}
          />
        </div>
      </div>
    </div>
  )
}

interface ExtraIconsProps {
  isTunModeAvailable: boolean
  isServiceOk: boolean
  tunUnavailableTooltip: string
  installServiceTooltip: string
  uninstallServiceTooltip: string
  onInstallService: () => void
  onUninstallService: () => void
}

export const TunModeExtraIcons = ({
  isTunModeAvailable,
  isServiceOk,
  tunUnavailableTooltip,
  installServiceTooltip,
  uninstallServiceTooltip,
  onInstallService,
  onUninstallService,
}: ExtraIconsProps) => {
  return (
    <>
      {!isTunModeAvailable && (
        <button
          type="button"
          className="text-xs ml-2 px-3 py-0.5 rounded-full border border-primary text-primary whitespace-nowrap hover:bg-primary/10 cursor-pointer transition-colors"
          onClick={onInstallService}
        >
          {installServiceTooltip}
        </button>
      )}
      {isServiceOk && (
        <button
          type="button"
          className="text-xs ml-2 px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
          onClick={onUninstallService}
        >
          {uninstallServiceTooltip}
        </button>
      )}
    </>
  )
}
