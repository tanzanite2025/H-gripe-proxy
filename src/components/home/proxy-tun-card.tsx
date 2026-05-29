import { HelpCircle, Monitor, type LucideIcon, ShieldCheck } from 'lucide-react'
import { useState, useMemo, memo, FC } from 'react'
import { useTranslation } from 'react-i18next'

import ProxyControlSwitches from '@/components/ui/proxy-control-switches'
import { Tooltip } from '@/components/tailwind/Tooltip'
import { useSystemProxyState, useSystemState, useVerge } from '@/hooks/system'
import { showNotice } from '@/services/notice-service'
import { cn } from '@/utils/cn'

const LOCAL_STORAGE_TAB_KEY = 'clash-verge-proxy-active-tab'

interface TabButtonProps {
  isActive: boolean
  onClick: () => void
  icon: LucideIcon
  label: string
  hasIndicator?: boolean
}

// Tab组件
const TabButton: FC<TabButtonProps> = memo(
  ({ isActive, onClick, icon: Icon, label, hasIndicator = false }) => (
    <div
      onClick={onClick}
      className={cn(
        'cursor-pointer px-3 h-8 flex items-center justify-center gap-2',
        'rounded-[20px] border-none transition-all duration-[250ms] ease-[cubic-bezier(0.16,1,0.3,1)]',
        'flex-1 max-w-[160px] relative',
        isActive
          ? 'bg-primary text-primary-contrast shadow-[0_2px_8px_-2px_rgba(var(--primary-main-rgb),0.3)]'
          : 'bg-transparent text-text-secondary hover:text-text-primary hover:bg-action-hover/5 hover:scale-[1.02]',
        'active:scale-[0.98]'
      )}
    >
      <Icon className="h-4 w-4" />
      <span
        className={cn(
          'text-[11px] tracking-[0.02em]',
          isActive ? 'font-black' : 'font-semibold'
        )}
      >
        {label}
      </span>
      {hasIndicator && (
        <div
          className={cn(
            'w-1.5 h-1.5 rounded-full absolute top-1.5 right-3',
            isActive ? 'bg-white' : 'bg-success'
          )}
        />
      )}
    </div>
  ),
)

interface TabDescriptionProps {
  activeTab: string
  description: string
  tooltipTitle: string
}

// 描述文本组件
const TabDescription: FC<TabDescriptionProps> = memo(
  ({ activeTab, description, tooltipTitle }) => (
    <div className="w-full flex items-center gap-3 px-1 animate-in fade-in duration-200">
      <div className="inline-flex items-center h-[18px] px-3 rounded-full bg-primary/8 text-primary text-[8px] font-mono font-black uppercase tracking-[0.1em] flex-shrink-0">
        {activeTab.toUpperCase()}
      </div>
      <p className="text-[9px] font-black uppercase tracking-[0.15em] text-text-secondary opacity-60 break-words leading-tight flex items-center gap-1">
        {description}
        <Tooltip title={tooltipTitle}>
          <HelpCircle className="h-3.5 w-3.5 opacity-70 flex-shrink-0 cursor-pointer" />
        </Tooltip>
      </p>
    </div>
  ),
)

export const ProxyTunCard: FC = () => {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<string>(
    () => localStorage.getItem(LOCAL_STORAGE_TAB_KEY) || 'system',
  )

  const { verge } = useVerge()
  const { isTunModeAvailable } = useSystemState()
  const { configState: systemProxyConfigState } = useSystemProxyState()

  const { enable_tun_mode } = verge ?? {}

  const handleError = (err: unknown) => {
    showNotice.error(err)
  }

  const handleTabChange = (tab: string) => {
    setActiveTab(tab)
    localStorage.setItem(LOCAL_STORAGE_TAB_KEY, tab)
  }

  const tabDescription = useMemo(() => {
    if (activeTab === 'system') {
      return {
        text: systemProxyConfigState
          ? t('home.components.proxyTun.status.systemProxyEnabled')
          : t('home.components.proxyTun.status.systemProxyDisabled'),
        tooltip: t('home.components.proxyTun.tooltips.systemProxy'),
      }
    } else {
      return {
        text: !isTunModeAvailable
          ? t('home.components.proxyTun.status.tunModeServiceRequired')
          : enable_tun_mode
            ? t('home.components.proxyTun.status.tunModeEnabled')
            : t('home.components.proxyTun.status.tunModeDisabled'),
        tooltip: t('home.components.proxyTun.tooltips.tunMode'),
      }
    }
  }, [
    activeTab,
    systemProxyConfigState,
    enable_tun_mode,
    isTunModeAvailable,
    t,
  ])

  return (
    <div className="flex flex-col w-full mt-1">
      {/* 模式选择按钮组 - 工业滑块选择器 */}
      <div className="flex items-center justify-between p-1 h-10 bg-action-hover/[0.02] border border-dashed border-divider rounded-3xl w-full">
        <TabButton
          isActive={activeTab === 'system'}
          onClick={() => handleTabChange('system')}
          icon={Monitor}
          label={t('settings.sections.system.toggles.systemProxy')}
          hasIndicator={systemProxyConfigState}
        />
        <TabButton
          isActive={activeTab === 'tun'}
          onClick={() => handleTabChange('tun')}
          icon={ShieldCheck}
          label={t('settings.sections.system.toggles.tunMode')}
          hasIndicator={enable_tun_mode && isTunModeAvailable}
        />
      </div>

      {/* 说明文本区域 - 微型 Badge */}
      <div className="w-full mt-3 flex justify-center overflow-visible">
        <TabDescription
          activeTab={activeTab}
          description={tabDescription.text}
          tooltipTitle={tabDescription.tooltip}
        />
      </div>

      {/* 底部开关组件容器 - dashed 虚线边框融入底板 */}
      <div className="mt-3 p-[6px_10px] bg-paper/40 border border-dashed border-divider rounded-[20px]">
        <ProxyControlSwitches
          onError={handleError}
          label={
            activeTab === 'system'
              ? t('settings.sections.system.toggles.systemProxy')
              : t('settings.sections.system.toggles.tunMode')
          }
          noRightPadding={true}
        />
      </div>
    </div>
  )
}
