import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Switch } from '@/components/tailwind/Switch'
import type { EgressIdentityConfig } from '@/services/coordinator'

interface Props {
  config: EgressIdentityConfig
  activeAssignmentCount: number
  domainPatternAssignmentCount: number
  onToggleEnabled: (enabled: boolean) => void
  onInitialize: () => void | Promise<void>
  onClearRules: () => void
}

export function EgressIdentityOverviewCard({
  config,
  activeAssignmentCount,
  domainPatternAssignmentCount,
  onToggleEnabled,
  onInitialize,
  onClearRules,
}: Props) {
  return (
    <>
      <div className="rounded-lg bg-blue-500 p-4 text-white">
        <p className="text-sm">
          代理软件会把应用、快捷方式和业务会话统一映射到稳定的出口身份，尽量让同一主体持续使用同一画像与节点。
        </p>
      </div>

      <Card variant="outlined">
        <div className="space-y-4 p-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-semibold">启用出口身份管理</p>
              <p className="mt-1 text-sm text-gray-500">
                把出口选择提升为“画像 + 规则 + 运行时 assignment”的统一模型。
              </p>
            </div>
            <Switch
              checked={config.enabled}
              onCheckedChange={onToggleEnabled}
            />
          </div>

          <div className="grid grid-cols-1 gap-3 md:grid-cols-5">
            <div className="rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40">
              <div className="text-2xl font-bold">{config.profiles.length}</div>
              <div className="mt-1 text-sm text-gray-500">出口画像</div>
            </div>
            <div className="rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40">
              <div className="text-2xl font-bold">{config.app_rules.length}</div>
              <div className="mt-1 text-sm text-gray-500">应用规则</div>
            </div>
            <div className="rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40">
              <div className="text-2xl font-bold">
                {config.shortcut_rules.length}
              </div>
              <div className="mt-1 text-sm text-gray-500">
                快捷方式规则
              </div>
            </div>
            <div className="rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40">
              <div className="text-2xl font-bold">{activeAssignmentCount}</div>
              <div className="mt-1 text-sm text-gray-500">
                运行时 assignment
              </div>
            </div>
            <div className="rounded-lg bg-purple-50 p-3 dark:bg-purple-950/30">
              <div className="text-2xl font-bold">
                {domainPatternAssignmentCount}
              </div>
              <div className="mt-1 text-sm text-gray-500">
                domain-pattern 回写
              </div>
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button size="small" variant="outlined" onClick={onInitialize}>
              初始化模板
            </Button>
            <Button size="small" variant="outlined" onClick={onClearRules}>
              清空规则
            </Button>
          </div>
        </div>
      </Card>
    </>
  )
}
