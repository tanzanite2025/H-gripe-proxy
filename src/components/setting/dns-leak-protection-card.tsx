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
      <Typography variant="subtitle2" sx={{ mb: 2, fontWeight: 600 }}>
        DNS 零泄漏防护
      </Typography>

      <Alert severity="info" sx={{ mb: 2, fontSize: '0.75rem' }}>
        DNS 零泄漏防护确保所有 DNS 查询都通过加密通道，防止 ISP 或中间人监控
      </Alert>

      <Box sx={{ mb: 2 }}>
        <Typography variant="caption" sx={{ mb: 1.5, color: 'text.secondary', display: 'block' }}>
          防护级别
        </Typography>
        <ToggleButtonGroup
          value={level}
          exclusive
          onChange={handleLevelChange}
          fullWidth
          sx={{ mb: 1.5 }}
        >
          <ToggleButton value="none" sx={{ fontSize: '0.75rem', py: 1 }}>
            <ShieldLowIcon sx={{ mr: 0.5, fontSize: '1rem' }} />
            无防护
          </ToggleButton>
          <ToggleButton value="basic" sx={{ fontSize: '0.75rem', py: 1 }}>
            <ShieldIcon sx={{ mr: 0.5, fontSize: '1rem' }} />
            基础
          </ToggleButton>
          <ToggleButton value="strict" sx={{ fontSize: '0.75rem', py: 1 }}>
            <SecurityIcon sx={{ mr: 0.5, fontSize: '1rem' }} />
            严格
          </ToggleButton>
          <ToggleButton value="paranoid" sx={{ fontSize: '0.75rem', py: 1 }}>
            <VerifiedIcon sx={{ mr: 0.5, fontSize: '1rem' }} />
            偏执
          </ToggleButton>
        </ToggleButtonGroup>

        <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
          {stats.levelName}
        </Typography>
      </Box>

      <Divider sx={{ my: 2 }} />

      <Box sx={{ mb: 2 }}>
        <Typography variant="caption" sx={{ mb: 1.5, color: 'text.secondary', display: 'block' }}>
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

      <Box sx={{ mb: 2 }}>
        <Typography variant="caption" sx={{ mb: 1.5, color: 'text.secondary', display: 'block' }}>
          防护特性
        </Typography>

        <List dense sx={{ py: 0 }}>
          {stats.features.map((feature, index) => (
            <ListItem key={index} sx={{ py: 0.5, px: 0 }}>
              <ListItemIcon sx={{ minWidth: 28 }}>
                <CheckIcon color="success" sx={{ fontSize: '1rem' }} />
              </ListItemIcon>
              <ListItemText 
                primary={feature}
                slotProps={{
                  primary: { variant: 'body2' }
                }}
              />
            </ListItem>
          ))}
        </List>
      </Box>

      <Divider sx={{ my: 2 }} />

      <Box>
        <Typography variant="caption" sx={{ mb: 1.5, color: 'text.secondary', display: 'block' }}>
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
              <Alert severity="error" icon={<ErrorIcon fontSize="small" />} sx={{ fontSize: '0.75rem' }}>
                检测到 DNS 泄漏！
              </Alert>
            ) : (
              <Alert severity="success" icon={<CheckIcon fontSize="small" />} sx={{ fontSize: '0.75rem' }}>
                未检测到 DNS 泄漏
              </Alert>
            )}

            {testResult.leakType.length > 0 && (
              <Box sx={{ mt: 1 }}>
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                  泄漏类型:
                </Typography>
                <Stack direction="row" spacing={0.5} sx={{ flexWrap: 'wrap', gap: 0.5 }}>
                  {testResult.leakType.map((type, index) => (
                    <Chip key={index} label={type} size="small" color="error" sx={{ fontSize: '0.7rem' }} />
                  ))}
                </Stack>
              </Box>
            )}

            {testResult.recommendations.length > 0 && (
              <Box sx={{ mt: 1 }}>
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                  建议:
                </Typography>
                <List dense sx={{ py: 0 }}>
                  {testResult.recommendations.map((rec, index) => (
                    <ListItem key={index} sx={{ py: 0.5, px: 0 }}>
                      <ListItemIcon sx={{ minWidth: 28 }}>
                        <WarningIcon color="warning" sx={{ fontSize: '1rem' }} />
                      </ListItemIcon>
                      <ListItemText 
                        primary={rec}
                        slotProps={{
                          primary: { variant: 'body2' }
                        }}
                      />
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
