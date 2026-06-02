/**
 * DNS 零泄漏防护配置卡片
 */

import { CheckCircle as CheckIcon, Loader2 as CircularProgress, Shield as ShieldIcon, ShieldAlert as ShieldLowIcon, ShieldCheck as SecurityIcon, ShieldOff as VerifiedIcon, AlertTriangle as WarningIcon } from 'lucide-react'
import { useState } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { List, ListItem, ListItemIcon, ListItemText } from '@/components/tailwind/List'
import { ToggleButton, ToggleButtonGroup } from '@/components/tailwind/ToggleButtonGroup'
import { testDnsLeak, type DnsLeakTestResult, type DnsRuntimeStatus } from '@/services/cmds'
import type { DnsLeakProtectionLevel } from '@/services/coordinator'
import {
  formatDNSLeakSignal,
  formatDNSRuntimeRisk,
} from '@/services/dns-leak-detection'
import {
  dnsLeakProtectionService,
} from '@/services/dns-leak-protection'
import { buildDnsLeakTestViewModel } from './dns-leak-test-view-model'
import { buildDnsRuntimeViewModel } from './dns-runtime-view-model'

interface Props {
  level: DnsLeakProtectionLevel
  runtimeStatus?: DnsRuntimeStatus
  onChange: (level: DnsLeakProtectionLevel) => void
}

export const DnsLeakProtectionCard = ({ level, runtimeStatus, onChange }: Props) => {
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<DnsLeakTestResult | null>(null)

  const previewDescription = dnsLeakProtectionService.getLevelDescription(level)
  const runtimeView = runtimeStatus
    ? buildDnsRuntimeViewModel(runtimeStatus)
    : null
  const runtimeFeatures = runtimeView?.leak.features ?? []
  const testView = testResult ? buildDnsLeakTestViewModel(testResult) : null

  const handleLevelChange = (
    _event: React.MouseEvent<HTMLElement>,
    value: string | string[],
  ) => {
    if (typeof value === 'string') {
      const newLevel = value as DnsLeakProtectionLevel
      onChange(newLevel)
    }
  }

  const handleTestLeak = async () => {
    setTesting(true)
    try {
      const result = await testDnsLeak()
      setTestResult(result)
    } catch (err) {
      console.error('DNS leak test failed:', err)
    } finally {
      setTesting(false)
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
      case 'very-high':
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
          {previewDescription.name}
        </div>
      </div>

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div className="mb-2">
        <div className="mb-1.5 block text-xs text-gray-500 dark:text-gray-400">
          后端确认的当前运行态
        </div>

        <div className="space-y-1.5">
          <div className="flex items-center gap-1">
            <div className="text-sm">安全级别:</div>
            <Chip
              icon={getSecurityIcon(runtimeView?.leak.security || 'unknown')}
              label={runtimeView?.leak.securityUnknownLabel ?? '未知'}
              size="small"
              color={runtimeView?.leak.securityColor ?? 'default'}
            />
          </div>

          <div className="flex items-center gap-1">
            <div className="text-sm">防护状态:</div>
            {runtimeView?.leak.safe === null || !runtimeView ? (
              <Chip label="未知" size="small" color="default" />
            ) : runtimeView.leak.safe ? (
              <Chip icon={<CheckIcon className="h-3 w-3" />} label="安全" size="small" color="success" />
            ) : (
              <Chip icon={<WarningIcon className="h-3 w-3" />} label="不安全" size="small" color="error" />
            )}
          </div>

          <div className="flex items-center gap-1">
            <div className="text-sm">防护级别:</div>
            <Chip
              label={runtimeView?.leak.levelUnknownLabel ?? '未知'}
              size="small"
              color={runtimeView?.leak.securityColor ?? 'default'}
            />
          </div>
        </div>
      </div>

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div className="mb-2">
        <div className="mb-1.5 block text-xs text-gray-500 dark:text-gray-400">
          运行态特性
        </div>

        <List dense className="py-0">
          {(runtimeFeatures.length > 0
            ? runtimeFeatures
            : ['暂未识别到后端运行态特性']
          ).map((feature) => (
            <ListItem key={feature} className="px-0 py-0.5">
              <ListItemIcon className="min-w-[28px]">
                <CheckIcon className="h-4 w-4 text-green-500" />
              </ListItemIcon>
              <ListItemText primary={<span className="text-sm">{feature}</span>} />
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

        {testResult && testView && (
          <div className="mt-2">
            <Alert severity={testView.alert.severity} className="text-xs">
              {testView.alert.message}
            </Alert>

            <div className="mt-1 space-y-1">
              <div className="flex items-center justify-between gap-2 text-xs text-gray-600 dark:text-gray-400">
                <span>风险等级</span>
                <Chip
                  size="small"
                  color={testView.riskLevel.color}
                  label={testView.riskLevel.label}
                />
              </div>
              <div className="flex items-center justify-between gap-2 text-xs text-gray-600 dark:text-gray-400">
                <span>结果判定</span>
                <Chip
                  size="small"
                  color={testView.assessment.color}
                  label={testView.assessment.label}
                />
              </div>
              <div className="flex items-center justify-between gap-2 text-xs text-gray-600 dark:text-gray-400">
                <span>结果置信度</span>
                <Chip
                  size="small"
                  color={testView.confidence.color}
                  label={testView.confidence.label}
                />
              </div>
              <div className="flex items-center justify-between gap-2 text-xs text-gray-600 dark:text-gray-400">
                <span>出口位置</span>
                <span>{testResult.ip_location}</span>
              </div>
              <div className="flex items-center justify-between gap-2 text-xs text-gray-600 dark:text-gray-400">
                <span>DNS 位置</span>
                <span>{testResult.dns_location ?? 'Unknown'}</span>
              </div>
              <div className="flex items-center justify-between gap-2 text-xs text-gray-600 dark:text-gray-400">
                <span>检测方式</span>
                <Chip
                  size="small"
                  color={testView.observationPath.color}
                  label={testView.observationPath.label}
                />
              </div>
            </div>

            {!testResult.location_comparable && (
              <Alert severity="info" className="mt-1 text-xs">
                当前 DNS 位置与出口位置尚不可直接比较，结论主要依赖现有外部观测与运行态风险信号。
              </Alert>
            )}

            {testResult.warnings.length > 0 && (
              <Alert severity="warning" className="mt-1 text-xs">
                {testResult.warnings.join('；')}
              </Alert>
            )}

            {testResult.error && (
              <Alert severity="warning" className="mt-1 text-xs">
                {testResult.error}
              </Alert>
            )}

            {testResult.observed_leak_type.length > 0 && (
              <div className="mt-1">
                <div className="mb-0.5 block text-xs text-gray-600 dark:text-gray-400">
                  外部观测信号:
                </div>
                <div className="flex flex-wrap gap-0.5">
                  {testResult.observed_leak_type.map((type) => (
                    <Chip key={type} label={formatDNSLeakSignal(type)} size="small" color="error" className="text-[0.7rem]" />
                  ))}
                </div>
              </div>
            )}

            {testResult.runtime_risk_type.length > 0 && (
              <div className="mt-1">
                <div className="mb-0.5 block text-xs text-gray-600 dark:text-gray-400">
                  运行态风险:
                </div>
                <div className="flex flex-wrap gap-0.5">
                  {testResult.runtime_risk_type.map((type) => (
                    <Chip key={type} label={formatDNSRuntimeRisk(type)} size="small" color="warning" className="text-[0.7rem]" />
                  ))}
                </div>
              </div>
            )}

            {testResult.dns_servers.length > 0 && (
              <div className="mt-1">
                <div className="mb-0.5 block text-xs text-gray-600 dark:text-gray-400">
                  DNS 服务器:
                </div>
                <List dense className="py-0">
                  {testResult.dns_servers.slice(0, 3).map((server) => (
                    <ListItem key={server.ip} className="px-0 py-0.5">
                      <ListItemIcon className="min-w-[28px]">
                        <CheckIcon className="h-4 w-4 text-green-500" />
                      </ListItemIcon>
                      <ListItemText
                        primary={
                          <span className="text-sm">
                            {server.ip}
                            {server.country ? ` · ${server.country}` : ''}
                            {server.city ? ` · ${server.city}` : ''}
                            {server.isp ? ` · ${server.isp}` : ''}
                          </span>
                        }
                      />
                    </ListItem>
                  ))}
                </List>
              </div>
            )}

            {testResult.recommendations.length > 0 && (
              <div className="mt-1">
                <div className="mb-0.5 block text-xs text-gray-600 dark:text-gray-400">
                  建议:
                </div>
                <List dense className="py-0">
                  {testResult.recommendations.map((rec) => (
                    <ListItem key={rec} className="px-0 py-0.5">
                      <ListItemIcon className="min-w-[28px]">
                        <WarningIcon className="h-4 w-4 text-yellow-500" />
                      </ListItemIcon>
                      <ListItemText primary={<span className="text-sm">{rec}</span>} />
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
