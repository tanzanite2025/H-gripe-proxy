import { Plus, Sparkles, Trash2 } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Button, Switch, TextField } from '@/components/tailwind'

import type { HoneypotDecoy, NewHoneypotDecoyInput } from '../security-honeypot-decoys'
import type { HoneypotDecoyStrategyProfile } from '../security-honeypot-decoy-strategy'

interface HoneypotDecoyPanelProps {
  honeypotDecoys: HoneypotDecoy[]
  activeDecoyId: string
  decoyPath: string
  onDecoyPathChange: (path: string) => void
  onActiveDecoyChange: (decoyId: string) => void
  onAddHoneypotDecoy: (input: NewHoneypotDecoyInput) => void
  onRemoveHoneypotDecoy: (decoyId: string) => void
  onHoneypotDecoyEnabledChange: (decoyId: string, enabled: boolean) => void
  onApplyHoneypotDecoyStrategy: (
    profile?: Partial<HoneypotDecoyStrategyProfile>,
  ) => void
  onDeployDecoy: () => void
  onCleanupDecoy: () => void
  onCheckDecoyAccess: () => void
}

export default function HoneypotDecoyPanel({
  honeypotDecoys,
  activeDecoyId,
  decoyPath,
  onDecoyPathChange,
  onActiveDecoyChange,
  onAddHoneypotDecoy,
  onRemoveHoneypotDecoy,
  onHoneypotDecoyEnabledChange,
  onApplyHoneypotDecoyStrategy,
  onDeployDecoy,
  onCleanupDecoy,
  onCheckDecoyAccess,
}: HoneypotDecoyPanelProps) {
  const enabledCount = honeypotDecoys.filter((decoy) => decoy.enabled).length

  const handleQuickAdd = () => {
    const nextIndex = honeypotDecoys.length + 1
    onAddHoneypotDecoy({
      name: `动态诱饵 ${nextIndex}`,
      path: `profiles/config_decoy_${nextIndex}.yaml`,
      enabled: true,
    })
  }

  return (
    <div className="p-4 bg-card border border-border rounded-lg">
      <div className="flex items-center justify-between gap-3 mb-4">
        <h3 className="text-sm font-semibold">配置文件欺骗</h3>
        <span className="text-xs text-muted-foreground">
          {enabledCount}/{honeypotDecoys.length} 启用
        </span>
      </div>

      <div className="space-y-4">
        <TextField
          label="当前诱饵路径"
          value={decoyPath}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onDecoyPathChange(event.target.value)
          }
          fullWidth
          helperText="编辑当前选中的诱饵路径；部署/检查会处理所有启用诱饵"
        />

        <div className="flex flex-wrap gap-2">
          <Button variant="default" onClick={onDeployDecoy}>
            部署启用项
          </Button>
          <Button variant="outline" onClick={onCheckDecoyAccess}>
            检查访问
          </Button>
          <Button variant="outline" onClick={onCleanupDecoy}>
            清除启用项
          </Button>
        </div>

        <div className="border border-border rounded-lg overflow-hidden">
          <div className="flex items-center justify-between gap-2 px-3 py-2 bg-secondary/40">
            <span className="text-xs font-semibold">动态诱饵</span>
            <div className="flex gap-2">
              <Button size="sm" variant="ghost" onClick={handleQuickAdd}>
                <Plus className="w-3.5 h-3.5" />
                新增
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => onApplyHoneypotDecoyStrategy()}
              >
                <Sparkles className="w-3.5 h-3.5" />
                应用策略
              </Button>
            </div>
          </div>

          <div className="divide-y divide-border">
            {honeypotDecoys.map((decoy) => {
              const isActive = decoy.id === activeDecoyId

              return (
                <div
                  key={decoy.id}
                  className={`flex items-center gap-3 px-3 py-2 ${
                    isActive ? 'bg-primary/10' : ''
                  }`}
                >
                  <button
                    type="button"
                    onClick={() => onActiveDecoyChange(decoy.id)}
                    className="min-w-0 flex-1 text-left"
                  >
                    <div className="flex items-center gap-2">
                      <span className="truncate text-xs font-semibold">
                        {decoy.name}
                      </span>
                      {isActive && (
                        <span className="shrink-0 rounded-full bg-primary px-2 py-0.5 text-[10px] text-white">
                          当前
                        </span>
                      )}
                    </div>
                    <p className="truncate text-[11px] text-muted-foreground">
                      {decoy.path}
                    </p>
                  </button>

                  <Switch
                    checked={decoy.enabled}
                    onCheckedChange={(enabled) =>
                      onHoneypotDecoyEnabledChange(decoy.id, enabled)
                    }
                    size="small"
                  />
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => onRemoveHoneypotDecoy(decoy.id)}
                    disabled={honeypotDecoys.length <= 1}
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    删除
                  </Button>
                </div>
              )
            })}
          </div>
        </div>
      </div>
    </div>
  )
}
