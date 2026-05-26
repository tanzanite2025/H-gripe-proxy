import { Box, Chip, Typography, alpha, useTheme } from '@mui/material'
import { useTranslation } from 'react-i18next'

import delayManager from '@/services/delay'

import { convertDelayColor } from '../utils/proxy-helpers'

interface ProxyInfoDisplayProps {
  proxy: any
  delay: number
  isGlobalMode: boolean
  isDirectMode: boolean
}

/**
 * 代理信息展示组件
 * 显示当前代理的详细信息和延迟
 */
export const ProxyInfoDisplay = ({
  proxy,
  delay,
  isGlobalMode,
  isDirectMode,
}: ProxyInfoDisplayProps) => {
  const { t } = useTranslation()
  const theme = useTheme()

  if (!proxy) {
    return (
      <Box sx={{ textAlign: 'center', py: 4 }}>
        <Typography variant="body1" color="text.secondary">
          {t('home.components.currentProxy.labels.noActiveNode')}
        </Typography>
      </Box>
    )
  }

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        p: 1.5,
        mb: 1.5,
        borderRadius: 2,
        bgcolor: alpha(theme.palette.action.hover, 0.02),
        border: '1px dashed',
        borderColor: 'divider',
      }}
    >
      <Box>
        <Typography variant="body1" sx={{ fontWeight: 'medium' }}>
          {proxy.name}
        </Typography>

        <Box sx={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap' }}>
          <Typography variant="caption" color="text.secondary" sx={{ mr: 1 }}>
            {proxy.type}
          </Typography>

          {/* 模式标签 */}
          {isGlobalMode && (
            <Chip
              size="small"
              label={t('home.components.currentProxy.labels.globalMode')}
              color="primary"
              sx={{ mr: 0.5 }}
            />
          )}
          {isDirectMode && (
            <Chip
              size="small"
              label={t('home.components.currentProxy.labels.directMode')}
              color="success"
              sx={{ mr: 0.5 }}
            />
          )}

          {/* 节点特性 */}
          {proxy.udp && (
            <Chip size="small" label="UDP" variant="outlined" sx={{ mr: 0.5 }} />
          )}
          {proxy.tfo && (
            <Chip size="small" label="TFO" variant="outlined" sx={{ mr: 0.5 }} />
          )}
          {proxy.xudp && (
            <Chip size="small" label="XUDP" variant="outlined" sx={{ mr: 0.5 }} />
          )}
          {proxy.mptcp && (
            <Chip size="small" label="MPTCP" variant="outlined" sx={{ mr: 0.5 }} />
          )}
          {proxy.smux && (
            <Chip size="small" label="SMUX" variant="outlined" sx={{ mr: 0.5 }} />
          )}
        </Box>
      </Box>

      {/* 显示延迟 */}
      {!isDirectMode && (
        <Chip
          size="small"
          label={delayManager.formatDelay(delay)}
          color={convertDelayColor(delay)}
        />
      )}
    </Box>
  )
}
