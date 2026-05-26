/**
 * DNS 智能分流配置卡片
 */

import { useState, useEffect } from 'react'
import {
  Box,
  Typography,
  ToggleButtonGroup,
  ToggleButton,
  Chip,
  Stack,
  Divider,
  Alert,
} from '@mui/material'
import {
  Speed as SpeedIcon,
  Security as SecurityIcon,
  Balance as BalanceIcon,
  Settings as SettingsIcon,
} from '@mui/icons-material'
import { dnsSmartRoutingService, type DnsRoutingMode } from '@/services/dns-smart-routing'

export const DnsRoutingCard = () => {
  const [mode, setMode] = useState<DnsRoutingMode>('balanced')
  const [stats, setStats] = useState({
    mode: 'balanced' as DnsRoutingMode,
    domesticDns: '',
    foreignDns: '',
    customRulesCount: 0,
  })

  useEffect(() => {
    // 初始化
    const config = dnsSmartRoutingService.getConfig()
    setMode(config.mode)
    updateStats()

    // 定期更新统计
    const interval = setInterval(updateStats, 5000)
    return () => clearInterval(interval)
  }, [])

  const updateStats = () => {
    const newStats = dnsSmartRoutingService.getStats()
    setStats(newStats)
  }

  const handleModeChange = (_event: React.MouseEvent<HTMLElement>, newMode: DnsRoutingMode) => {
    if (newMode !== null) {
      setMode(newMode)
      dnsSmartRoutingService.setMode(newMode)
      updateStats()
    }
  }

  const getModeDescription = (mode: DnsRoutingMode): string => {
    switch (mode) {
      case 'speed':
        return '全部使用国内 UDP DNS，延迟最低（10-30ms）'
      case 'privacy':
        return '全部使用 Cloudflare DoH，隐私保护最强'
      case 'balanced':
        return '国内域名用 UDP，国外域名用 DoH，平衡速度和隐私'
      case 'custom':
        return '自定义 DNS 配置和规则'
    }
  }

  const getModeColor = (mode: DnsRoutingMode): 'success' | 'info' | 'warning' | 'default' => {
    switch (mode) {
      case 'speed':
        return 'success'
      case 'privacy':
        return 'info'
      case 'balanced':
        return 'warning'
      case 'custom':
        return 'default'
    }
  }

  return (
    <Box>
      <Typography variant="h6" sx={{ mb: 2, fontWeight: 'bold' }}>
        DNS 智能分流
      </Typography>

      <Alert severity="info" sx={{ mb: 2 }}>
        智能分流可根据域名类型自动选择最优 DNS 服务器，提升解析速度并保护隐私
      </Alert>

      <Box sx={{ mb: 3 }}>
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
          分流模式
        </Typography>
        <ToggleButtonGroup
          value={mode}
          exclusive
          onChange={handleModeChange}
          fullWidth
          sx={{ mb: 2 }}
        >
          <ToggleButton value="speed">
            <SpeedIcon sx={{ mr: 1 }} />
            速度优先
          </ToggleButton>
          <ToggleButton value="balanced">
            <BalanceIcon sx={{ mr: 1 }} />
            平衡模式
          </ToggleButton>
          <ToggleButton value="privacy">
            <SecurityIcon sx={{ mr: 1 }} />
            隐私优先
          </ToggleButton>
          <ToggleButton value="custom">
            <SettingsIcon sx={{ mr: 1 }} />
            自定义
          </ToggleButton>
        </ToggleButtonGroup>

        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {getModeDescription(mode)}
        </Typography>
      </Box>

      <Divider sx={{ my: 2 }} />

      <Box>
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
          当前配置
        </Typography>

        <Stack spacing={1.5}>
          <Box>
            <Typography variant="caption" color="text.secondary">
              当前模式
            </Typography>
            <Box sx={{ mt: 0.5 }}>
              <Chip
                label={
                  mode === 'speed'
                    ? '速度优先'
                    : mode === 'privacy'
                      ? '隐私优先'
                      : mode === 'balanced'
                        ? '平衡模式'
                        : '自定义'
                }
                color={getModeColor(mode)}
                size="small"
              />
            </Box>
          </Box>

          <Box>
            <Typography variant="caption" color="text.secondary">
              国内域名 DNS
            </Typography>
            <Typography variant="body2" sx={{ mt: 0.5 }}>
              {stats.domesticDns || '未配置'}
            </Typography>
          </Box>

          <Box>
            <Typography variant="caption" color="text.secondary">
              国外域名 DNS
            </Typography>
            <Typography variant="body2" sx={{ mt: 0.5 }}>
              {stats.foreignDns || '未配置'}
            </Typography>
          </Box>

          {stats.customRulesCount > 0 && (
            <Box>
              <Typography variant="caption" color="text.secondary">
                自定义规则
              </Typography>
              <Typography variant="body2" sx={{ mt: 0.5 }}>
                {stats.customRulesCount} 条规则
              </Typography>
            </Box>
          )}
        </Stack>
      </Box>

      <Divider sx={{ my: 2 }} />

      <Box>
        <Typography variant="caption" color="text.secondary">
          性能提示
        </Typography>
        <Stack direction="row" spacing={1} sx={{ mt: 1 }}>
          {mode === 'speed' && (
            <>
              <Chip label="延迟: 10-30ms" size="small" color="success" />
              <Chip label="隐私: 低" size="small" />
            </>
          )}
          {mode === 'privacy' && (
            <>
              <Chip label="延迟: 30-80ms" size="small" />
              <Chip label="隐私: 高" size="small" color="success" />
            </>
          )}
          {mode === 'balanced' && (
            <>
              <Chip label="延迟: 20-40ms" size="small" color="success" />
              <Chip label="隐私: 中" size="small" color="warning" />
            </>
          )}
        </Stack>
      </Box>
    </Box>
  )
}
