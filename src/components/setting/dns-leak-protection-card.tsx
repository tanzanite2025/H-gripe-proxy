/**
 * DNS 零泄漏防护配置卡片
 */

import { AlertCircle as ErrorIcon, CheckCircle as CheckIcon, Loader2 as CircularProgress, Shield as ShieldIcon, ShieldAlert as ShieldLowIcon, ShieldCheck as SecurityIcon, ShieldOff as VerifiedIcon, AlertTriangle as WarningIcon } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { List, ListItem, ListItemIcon, ListItemText } from '@/components/tailwind/List'
import { ToggleButton, ToggleButtonGroup } from '@/components/tailwind/ToggleButtonGroup'
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
    <div>
      <div className="mb-2 text-sm font-semibold">
        DNS 零泄漏防护
      </div>

      <Alert severity="info" className="mb-2 text-xs">
        DNS 零泄漏防护确保所有 DNS 查询都通过加密通道，防止 ISP 或中间人监控
      </Alert>

      <div className="mb-2">
        <div className="mb-1.5 block text-xs text-gray-500 dark:text-gray-400">
          防护级别
        </div>
        <ToggleButtonGroup
          value={level}
          exclusive
          onChange={handleLevelChange}
          fullWidth
          className="mb-1.5"
        >
          <ToggleButton value="none" className="py-1 text-xs">
            <ShieldLowIcon className="mr-0.5 h-4 w-4" />
            无防护
          </ToggleButton>
          <ToggleButton value="basic" className="py-1 text-xs">
            <ShieldIcon className="mr-0.5 h-4 w-4" />
            基础
          </ToggleButton>
          <ToggleButton value="strict" className="py-1 text-xs">
            <SecurityIcon className="mr-0.5 h-4 w-4" />
            严格
          </ToggleButton>
          <ToggleButton value="paranoid" className="py-1 text-xs">
            <VerifiedIcon className="mr-0.5 h-4 w-4" />
            偏执
          </ToggleButton>
        </ToggleButtonGroup>

        <div className="block text-xs text-gray-600 dark:text-gray-400">
          {stats.levelName}
        </div>
      </div>

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div className="mb-2">
        <div className="mb-1.5 block text-xs text-gray-500 dark:text-gray-400">
          当前状态
        </div>

        <div className="space-y-1.5">
          <div className="flex items-center gap-1">
            <div className="text-sm">安全级别:</div>
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
          </div>

          <div className="flex items-center gap-1">
            <div className="text-sm">防护状态:</div>
            {stats.safe ? (
              <Chip icon={<CheckIcon className="h-3 w-3" />} label="安全" size="small" color="success" />
            ) : (
              <Chip icon={<WarningIcon className="h-3 w-3" />} label="不安全" size="small" color="error" />
            )}
          </div>
        </div>
      </div>

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div className="mb-2">
        <div className="mb-1.5 block text-xs text-gray-500 dark:text-gray-400">
          防护特性
        </div>

        <List dense className="py-0">
          {stats.features.map((feature, index) => (
            <ListItem key={index} className="px-0 py-0.5">
              <ListItemIcon className="min-w-[28px]">
                <CheckIcon className="h-4 w-4 text-green-500" />
              </ListItemIcon>
              <ListItemText 
                primary={feature}
                primaryTypographyProps={{ className: 'text-sm' }}
              />
            </ListItem>
          ))}
        </List>
      </div>

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div>
        <div className="mb-1.5 block text-xs text-gray-500 dark:text-gray-400">
          DNS 泄漏测试
        </div>

        <Button
          variant="outlined"
          fullWidth
          onClick={handleTestLeak}
          disabled={testing}
          startIcon={testing ? <CircularProgress className="h-4 w-4 animate-spin" /> : undefined}
        >
          {testing ? '测试中...' : '开始测试'}
        </Button>

        {testResult && (
          <div className="mt-2">
            {testResult.hasLeak ? (
              <Alert severity="error" icon={<ErrorIcon className="h-4 w-4" />} className="text-xs">
                检测到 DNS 泄漏！
              </Alert>
            ) : (
              <Alert severity="success" icon={<CheckIcon className="h-4 w-4" />} className="text-xs">
                未检测到 DNS 泄漏
              </Alert>
            )}

            {testResult.leakType.length > 0 && (
              <div className="mt-1">
                <div className="mb-0.5 block text-xs text-gray-600 dark:text-gray-400">
                  泄漏类型:
                </div>
                <div className="flex flex-wrap gap-0.5">
                  {testResult.leakType.map((type, index) => (
                    <Chip key={index} label={type} size="small" color="error" className="text-[0.7rem]" />
                  ))}
                </div>
              </div>
            )}

            {testResult.recommendations.length > 0 && (
              <div className="mt-1">
                <div className="mb-0.5 block text-xs text-gray-600 dark:text-gray-400">
                  建议:
                </div>
                <List dense className="py-0">
                  {testResult.recommendations.map((rec, index) => (
                    <ListItem key={index} className="px-0 py-0.5">
                      <ListItemIcon className="min-w-[28px]">
                        <WarningIcon className="h-4 w-4 text-yellow-500" />
                      </ListItemIcon>
                      <ListItemText 
                        primary={rec}
                        primaryTypographyProps={{ className: 'text-sm' }}
                      />
                    </ListItem>
                  ))}
                </List>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
