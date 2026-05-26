/**
 * DNS 零泄漏防护配置卡片
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
  Button,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  CircularProgress,
} from '@mui/material'
import {
  Shield as ShieldIcon,
  ShieldOutlined as ShieldLowIcon,
  Security as SecurityIcon,
  VerifiedUser as VerifiedIcon,
  CheckCircle as CheckIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
} from '@mui/icons-material'
import {
  dnsLeakProtectionService,
  type DnsLeakProtectionLevel,
} from '@/services/dns-leak-protection'

export const DnsLeakProtectionCard = () => {
  const [level, setLevel] = useState<DnsLeakProtectionLevel>('basic')
  const [stats, setStats] = useState({
    level: 'basic' as DnsLeakProtectionLevel,
    levelName: '基础防护',
    security: 'medium',
    features: [] as string[],
    safe: true,
  })
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<{
    hasLeak: boolean
    leakType: string[]
    recommendations: string[]
  } | null>(null)

  useEffect(() => {
    // 初始化
    const config = dnsLeakProtectionService.getConfig()
    setLevel(config.level)
    updateStats()
  }, [])

  const updateStats = () => {
    const newStats = dnsLeakProtectionService.getStats()
    setStats(newStats)
  }

  const handleLevelChange = (
    _event: React.MouseEvent<HTMLElement>,
    newLevel: DnsLeakProtectionLevel,
  ) => {
    if (newLevel !== null) {
      setLevel(newLevel)
      dnsLeakProtectionService.setLevel(newLevel)
      updateStats()
    }
  }

  const handleTestLeak = async () => {
    setTesting(true)
    try {
      const result = await dnsLeakProtectionService.testDnsLeak()
      setTestResult(result)
    } catch (err) {
      console.error('DNS leak test failed:', err)
    } finally {
      setTesting(false)
    }
  }

  const getSecurityColor = (
    security: string,
  ): 'error' | 'warning' | 'info' | 'success' => {
    switch (security) {
      case 'low':
        return 'error'
      case 'medium':
        return 'warning'
      case 'high':
        return 'info'
      case 'maximum':
        return 'success'
      default:
        return 'info'
    }
  }

  const getSecurityIcon = (security: string) => {
    switch (security) {
      case 'low':
        return <ShieldLowIcon />
      case 'medium':
        return <ShieldIcon />
      case 'high':
        return <SecurityIcon />
      case 'maximum':
        return <VerifiedIcon />
      default:
        return <ShieldIcon />
    }
  }

  return (
    <Box>
      <Typography variant="h6" sx={{ mb: 2, fontWeight: 'bold' }}>
        DNS 零泄漏防护
      </Typography>

      <Alert severity="info" sx={{ mb: 2 }}>
        DNS 零泄漏防护确保所有 DNS 查询都通过加密通道，防止 ISP 或中间人监控
      </Alert>

      <Box sx={{ mb: 3 }}>
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
          防护级别
        </Typography>
        <ToggleButtonGroup
          value={level}
          exclusive
          onChange={handleLevelChange}
          fullWidth
          sx={{ mb: 2 }}
        >
          <ToggleButton value="none">
            <ShieldLowIcon sx={{ mr: 1 }} />
            无防护
          </ToggleButton>
          <ToggleButton value="basic">
            <ShieldIcon sx={{ mr: 1 }} />
            基础
          </ToggleButton>
          <ToggleButton value="strict">
            <SecurityIcon sx={{ mr: 1 }} />
            严格
          </ToggleButton>
          <ToggleButton value="paranoid">
            <VerifiedIcon sx={{ mr: 1 }} />
            偏执
          </ToggleButton>
        </ToggleButtonGroup>

        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {stats.levelName}
        </Typography>
      </Box>

      <Divider sx={{ my: 2 }} />

      <Box sx={{ mb: 3 }}>
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
          当前状态
        </Typography>

        <Stack spacing={1.5}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Typography variant="body2">安全级别:</Typography>
            <Chip
              icon={getSecurityIcon(stats.security)}
              label={
                stats.security === 'low'
                  ? '低'
                  : stats.security === 'medium'
                    ? '中'
                    : stats.security === 'high'
                      ? '高'
                      : '最高'
              }
              size="small"
              color={getSecurityColor(stats.security)}
            />
          </Box>

          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Typography variant="body2">防护状态:</Typography>
            {stats.safe ? (
              <Chip icon={<CheckIcon />} label="安全" size="small" color="success" />
            ) : (
              <Chip icon={<WarningIcon />} label="不安全" size="small" color="error" />
            )}
          </Box>
        </Stack>
      </Box>

      <Divider sx={{ my: 2 }} />

      <Box sx={{ mb: 3 }}>
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
          防护特性
        </Typography>

        <List dense>
          {stats.features.map((feature, index) => (
            <ListItem key={index}>
              <ListItemIcon sx={{ minWidth: 36 }}>
                <CheckIcon color="success" fontSize="small" />
              </ListItemIcon>
              <ListItemText primary={feature} />
            </ListItem>
          ))}
        </List>
      </Box>

      <Divider sx={{ my: 2 }} />

      <Box>
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
          DNS 泄漏测试
        </Typography>

        <Button
          variant="outlined"
          fullWidth
          onClick={handleTestLeak}
          disabled={testing}
          startIcon={testing ? <CircularProgress size={16} /> : undefined}
        >
          {testing ? '测试中...' : '开始测试'}
        </Button>

        {testResult && (
          <Box sx={{ mt: 2 }}>
            {testResult.hasLeak ? (
              <Alert severity="error" icon={<ErrorIcon />}>
                检测到 DNS 泄漏！
              </Alert>
            ) : (
              <Alert severity="success" icon={<CheckIcon />}>
                未检测到 DNS 泄漏
              </Alert>
            )}

            {testResult.leakType.length > 0 && (
              <Box sx={{ mt: 1 }}>
                <Typography variant="caption" color="text.secondary">
                  泄漏类型:
                </Typography>
                <Stack direction="row" spacing={1} sx={{ mt: 0.5 }}>
                  {testResult.leakType.map((type, index) => (
                    <Chip key={index} label={type} size="small" color="error" />
                  ))}
                </Stack>
              </Box>
            )}

            {testResult.recommendations.length > 0 && (
              <Box sx={{ mt: 1 }}>
                <Typography variant="caption" color="text.secondary">
                  建议:
                </Typography>
                <List dense>
                  {testResult.recommendations.map((rec, index) => (
                    <ListItem key={index}>
                      <ListItemIcon sx={{ minWidth: 36 }}>
                        <WarningIcon color="warning" fontSize="small" />
                      </ListItemIcon>
                      <ListItemText primary={rec} />
                    </ListItem>
                  ))}
                </List>
              </Box>
            )}
          </Box>
        )}
      </Box>
    </Box>
  )
}
