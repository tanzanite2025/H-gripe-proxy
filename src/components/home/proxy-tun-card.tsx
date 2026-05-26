import {
  ComputerRounded,
  TroubleshootRounded,
  HelpOutlineRounded,
  SvgIconComponent,
} from '@mui/icons-material'
import {
  Box,
  Typography,
  Stack,
  Paper,
  Tooltip,
  alpha,
  useTheme,
  Fade,
} from '@mui/material'
import { useState, useMemo, memo, FC } from 'react'
import { useTranslation } from 'react-i18next'

import ProxyControlSwitches from '@/components/shared/proxy-control-switches'
import { useSystemProxyState } from '@/hooks/use-system-proxy-state'
import { useSystemState } from '@/hooks/use-system-state'
import { useVerge } from '@/hooks/use-verge'
import { showNotice } from '@/services/notice-service'

const LOCAL_STORAGE_TAB_KEY = 'clash-verge-proxy-active-tab'

interface TabButtonProps {
  isActive: boolean
  onClick: () => void
  icon: SvgIconComponent
  label: string
  hasIndicator?: boolean
}

// Tab组件
const TabButton: FC<TabButtonProps> = memo(
  ({ isActive, onClick, icon: Icon, label, hasIndicator = false }) => (
    <Box
      onClick={onClick}
      sx={(theme) => ({
        cursor: 'pointer',
        px: 1.5,
        height: 32,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 0.8,
        bgcolor: isActive ? 'primary.main' : 'transparent',
        color: isActive ? 'primary.contrastText' : 'text.secondary',
        borderRadius: '20px',
        border: 'none',
        boxShadow: isActive ? '0 2px 8px -2px rgba(var(--primary-main-rgb), 0.3)' : 'none',
        transition: 'all 0.25s cubic-bezier(0.16, 1, 0.3, 1)',
        flex: 1,
        maxWidth: 160,
        position: 'relative',
        '&:hover': {
          color: isActive ? 'primary.contrastText' : 'text.primary',
          bgcolor: isActive ? 'primary.main' : alpha(theme.palette.action.hover, 0.05),
          transform: isActive ? 'none' : 'scale(1.02)',
        },
        '&:active': {
          transform: 'scale(0.98)',
        },
      })}
    >
      <Icon fontSize="small" />
      <Typography
        variant="body2"
        sx={{
          fontWeight: isActive ? 900 : 600,
          fontSize: '11px',
          letterSpacing: '0.02em',
        }}
      >
        {label}
      </Typography>
      {hasIndicator && (
        <Box
          sx={{
            width: 6,
            height: 6,
            borderRadius: '50%',
            bgcolor: isActive ? '#fff' : 'success.main',
            position: 'absolute',
            top: 6,
            right: 12,
          }}
        />
      )}
    </Box>
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
    <Fade in={true} timeout={200}>
      <Box
        sx={{
          width: '100%',
          display: 'flex',
          alignItems: 'center',
          gap: 1.2,
          px: 0.5,
        }}
      >
        <Box
          sx={(theme) => ({
            display: 'inline-flex',
            alignItems: 'center',
            height: 18,
            px: 1.2,
            borderRadius: '9999px',
            bgcolor: alpha(theme.palette.primary.main, 0.08),
            color: 'primary.main',
            fontSize: '8px',
            fontFamily: 'monospace',
            fontWeight: 900,
            textTransform: 'uppercase',
            letterSpacing: '0.1em',
            flexShrink: 0,
          })}
        >
          {activeTab.toUpperCase()}
        </Box>
        <Typography
          variant="caption"
          sx={(theme) => ({
            fontSize: '9px',
            fontWeight: 900,
            textTransform: 'uppercase',
            letterSpacing: '0.15em',
            color: 'text.secondary',
            opacity: 0.6,
            wordBreak: 'break-word',
            lineHeight: 1.2,
            display: 'flex',
            alignItems: 'center',
            gap: 0.5,
          })}
        >
          {description}
          <Tooltip title={tooltipTitle}>
            <HelpOutlineRounded
              sx={{ fontSize: 12, opacity: 0.7, flexShrink: 0, cursor: 'pointer' }}
            />
          </Tooltip>
        </Typography>
      </Box>
    </Fade>
  ),
)

export const ProxyTunCard: FC = () => {
  const { t } = useTranslation()
  const theme = useTheme()
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
    <Box sx={{ display: 'flex', flexDirection: 'column', width: '100%', mt: 0.5 }}>
      {/* 模式选择按钮组 - 工业滑块选择器 */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          p: '4px',
          height: 40,
          bgcolor: alpha(theme.palette.action.hover, 0.02),
          border: '1px dashed',
          borderColor: 'divider',
          borderRadius: '24px',
          width: '100%',
          boxSizing: 'border-box',
        }}
      >
        <TabButton
          isActive={activeTab === 'system'}
          onClick={() => handleTabChange('system')}
          icon={ComputerRounded}
          label={t('settings.sections.system.toggles.systemProxy')}
          hasIndicator={systemProxyConfigState}
        />
        <TabButton
          isActive={activeTab === 'tun'}
          onClick={() => handleTabChange('tun')}
          icon={TroubleshootRounded}
          label={t('settings.sections.system.toggles.tunMode')}
          hasIndicator={enable_tun_mode && isTunModeAvailable}
        />
      </Box>

      {/* 说明文本区域 - 微型 Badge */}
      <Box
        sx={{
          width: '100%',
          mt: 1.5,
          display: 'flex',
          justifyContent: 'center',
          overflow: 'visible',
        }}
      >
        <TabDescription
          activeTab={activeTab}
          description={tabDescription.text}
          tooltipTitle={tabDescription.tooltip}
        />
      </Box>

      {/* 底部开关组件容器 - dashed 虚线边框融入底板 */}
      <Box
        sx={{
          mt: 1.5,
          p: '6px 10px',
          bgcolor: alpha(theme.palette.background.paper, 0.4),
          border: '1px dashed',
          borderColor: 'divider',
          borderRadius: '20px',
        }}
      >
        <ProxyControlSwitches
          onError={handleError}
          label={
            activeTab === 'system'
              ? t('settings.sections.system.toggles.systemProxy')
              : t('settings.sections.system.toggles.tunMode')
          }
          noRightPadding={true}
        />
      </Box>
    </Box>
  )
}
