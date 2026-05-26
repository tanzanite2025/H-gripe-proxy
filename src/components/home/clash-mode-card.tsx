import {
  DirectionsRounded,
  LanguageRounded,
  MultipleStopRounded,
} from '@mui/icons-material'
import { Box, Paper, Stack, Typography, alpha, useTheme } from '@mui/material'
import { useLockFn } from 'ahooks'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { closeAllConnections } from 'tauri-plugin-mihomo-api'

import { useVerge } from '@/hooks/system'
import {
  useAppRefreshers,
  useClashConfigData,
  useCoreDataStatus,
} from '@/providers/app-data-context'
import { patchClashMode } from '@/services/cmds'
import type { TranslationKey } from '@/types/generated/i18n-keys'

const CLASH_MODES = ['rule', 'global', 'direct'] as const
type ClashMode = (typeof CLASH_MODES)[number]

const isClashMode = (mode: string): mode is ClashMode =>
  (CLASH_MODES as readonly string[]).includes(mode)

const MODE_META: Record<
  ClashMode,
  { label: TranslationKey; description: TranslationKey }
> = {
  rule: {
    label: 'home.components.clashMode.labels.rule',
    description: 'home.components.clashMode.descriptions.rule',
  },
  global: {
    label: 'home.components.clashMode.labels.global',
    description: 'home.components.clashMode.descriptions.global',
  },
  direct: {
    label: 'home.components.clashMode.labels.direct',
    description: 'home.components.clashMode.descriptions.direct',
  },
}

export const ClashModeCard = () => {
  const { t } = useTranslation()
  const theme = useTheme()
  const { verge } = useVerge()
  const { clashConfig } = useClashConfigData()
  const { isCoreDataPending } = useCoreDataStatus()
  const { refreshClashConfig } = useAppRefreshers()

  // 支持的模式列表
  const modeList = CLASH_MODES

  // 直接使用API返回的模式，不维护本地状态
  const currentMode = clashConfig?.mode?.toLowerCase()
  const currentModeKey =
    typeof currentMode === 'string' && isClashMode(currentMode)
      ? currentMode
      : undefined

  const modeDescription = useMemo(() => {
    if (currentModeKey) {
      return t(MODE_META[currentModeKey].description)
    }
    if (isCoreDataPending) {
      return '\u00A0'
    }
    return t('home.components.clashMode.errors.communication')
  }, [currentModeKey, isCoreDataPending, t])

  // 模式图标映射
  const modeIcons = useMemo(
    () => ({
      rule: <MultipleStopRounded fontSize="small" />,
      global: <LanguageRounded fontSize="small" />,
      direct: <DirectionsRounded fontSize="small" />,
    }),
    [],
  )

  // 切换模式的处理函数
  const onChangeMode = useLockFn(async (mode: ClashMode) => {
    if (mode === currentModeKey) return
    if (verge?.auto_close_connection) {
      closeAllConnections()
    }

    try {
      await patchClashMode(mode)
      // 使用共享的刷新方法
      refreshClashConfig()
    } catch (error) {
      console.error('Failed to change mode:', error)
    }
  })

  // 按钮样式
  const buttonStyles = (mode: ClashMode) => {
    const isActive = mode === currentModeKey
    return {
      cursor: 'pointer',
      px: 1.5,
      height: 32,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 0.8,
      bgcolor: isActive ? 'primary.main' : 'transparent',
      color: isActive ? 'primary.contrastText' : 'text.secondary',
      borderRadius: '20px', // 内部项采用 20px 圆角
      border: 'none',
      boxShadow: isActive ? '0 2px 8px -2px rgba(var(--primary-main-rgb), 0.3)' : 'none',
      transition: 'all 0.25s cubic-bezier(0.16, 1, 0.3, 1)',
      flex: 1,
      maxWidth: 160,
      '&:hover': {
        color: isActive ? 'primary.contrastText' : 'text.primary',
        bgcolor: isActive ? 'primary.main' : alpha(theme.palette.action.hover, 0.05),
        transform: isActive ? 'none' : 'scale(1.02)',
      },
      '&:active': {
        transform: 'scale(0.98)',
      },
    }
  }

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
          borderRadius: '24px', // 胶囊型框
          width: '100%',
          boxSizing: 'border-box',
        }}
      >
        {modeList.map((mode) => (
          <Box
            key={mode}
            onClick={() => onChangeMode(mode)}
            sx={buttonStyles(mode)}
          >
            {modeIcons[mode]}
            <Typography
              variant="body2"
              sx={{
                textTransform: 'capitalize',
                fontWeight: mode === currentModeKey ? 900 : 600,
                fontSize: '11px',
                letterSpacing: '0.02em',
              }}
            >
              {t(MODE_META[mode].label)}
            </Typography>
          </Box>
        ))}
      </Box>

      {/* 说明文本区域 - 微型 Badge 元数据排版 */}
      <Box
        sx={{
          width: '100%',
          mt: 1.5,
          display: 'flex',
          alignItems: 'center',
          gap: 1.2,
          px: 0.5,
        }}
      >
        <Box
          sx={{
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
          }}
        >
          {currentModeKey || 'INFO'}
        </Box>
        <Typography
          variant="caption"
          sx={{
            fontSize: '9px',
            fontWeight: 900,
            textTransform: 'uppercase',
            letterSpacing: '0.15em',
            color: 'text.secondary',
            opacity: 0.6,
            wordBreak: 'break-word',
            lineHeight: 1.2,
          }}
        >
          {modeDescription}
        </Typography>
      </Box>
    </Box>
  )
}
