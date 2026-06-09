import {
  Checkbox,
  ListItemButton,
  ListItemText,
} from '@/components/tailwind'
import { cn } from '@/utils/cn'

import type { CandidateOption } from './types'

interface StrategyPoolCandidateGridProps {
  candidateOptions: CandidateOption[]
  loading: boolean
  selectedNameSet: ReadonlySet<string>
  onToggleSelected: (name: string, checked?: boolean) => void
}

export function StrategyPoolCandidateGrid({
  candidateOptions,
  loading,
  selectedNameSet,
  onToggleSelected,
}: StrategyPoolCandidateGridProps) {
  if (loading) {
    return (
      <div className="px-4 pb-2 pt-1 text-xs text-text-secondary">
        正在读取当前策略池已保存的成员...
      </div>
    )
  }

  if (candidateOptions.length === 0) {
    return (
      <div className="px-4 py-10 text-center text-sm text-text-secondary">
        没有匹配到可加入的节点。
      </div>
    )
  }

  return (
    <div className="max-h-[50vh] overflow-y-auto pr-1">
      <div className="grid grid-cols-3 items-start gap-2.5">
        {candidateOptions.map((option) => {
          const checked = selectedNameSet.has(option.name)

          return (
            <ListItemButton
              key={option.name}
              selected={checked}
              className={cn(
                'self-start items-start rounded-xl border border-white/8 px-3 py-2',
                checked ? 'bg-primary/10' : 'bg-black/10 hover:bg-white/8',
              )}
              onClick={() => onToggleSelected(option.name)}
            >
              <div
                className="mr-2 mt-0.5"
                onClick={(event) => event.stopPropagation()}
              >
                <Checkbox
                  checked={checked}
                  onChange={(_, nextChecked) =>
                    onToggleSelected(option.name, nextChecked)
                  }
                />
              </div>

              <ListItemText
                title={option.name}
                slotProps={{
                  secondary: {
                    className:
                      'mt-0.5 text-[11px] uppercase tracking-wide text-text-secondary',
                  },
                }}
                primary={
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="truncate text-sm font-semibold text-text-primary">
                      {option.name}
                    </span>
                    {option.provider ? (
                      <span className="rounded border border-white/10 px-1.5 py-0.5 text-[10px] text-text-secondary">
                        {option.provider}
                      </span>
                    ) : null}
                    {option.isGroup ? (
                      <span className="rounded border border-amber-500/30 px-1.5 py-0.5 text-[10px] text-amber-300">
                        组
                      </span>
                    ) : null}
                  </div>
                }
                secondary={
                  <span className="text-[11px] uppercase tracking-wide text-text-secondary">
                    {option.type}
                  </span>
                }
              />
            </ListItemButton>
          )
        })}
      </div>
    </div>
  )
}
