import { Plus, SlidersHorizontal } from 'lucide-react'

import { Box, Button, Chip } from '@/components/tailwind'
import { cn } from '@/utils/cn'

import type { ManagedStrategyPool, StrategyPoolGroupRef } from './types'

const SECTION_TITLE = '策略池'
const SECTION_DESCRIPTION =
  '策略池独立于上面的单节点出口区。你手动把节点加入池内，后续再给应用内启动或专用链路使用。'
const EMPTY_TEXT =
  '当前还没有软件自管的策略池。这里只显示你自己创建和维护的策略池，不再读取订阅自带分组。'
const CONFIG_NOT_READY_TEXT =
  '当前策略池配置文件还没准备好，暂时不能创建或编辑策略池。'

interface StrategyPoolsSectionProps {
  className?: string
  configReady: boolean
  pools: ManagedStrategyPool[]
  onCreate: () => void
  onEdit: (group: StrategyPoolGroupRef) => void
}

export function StrategyPoolsSection({
  className,
  configReady,
  pools,
  onCreate,
  onEdit,
}: StrategyPoolsSectionProps) {
  return (
    <Box
      className={cn(
        'mx-3 mb-2 rounded-2xl border border-white/10 bg-white/5 px-3 py-3',
        className,
      )}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-sm font-semibold text-text-primary">
            {SECTION_TITLE}
          </div>
          <div className="mt-0.5 text-xs text-text-secondary">
            {SECTION_DESCRIPTION}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Chip
            size="small"
            variant="outlined"
            color="primary"
            label={`已管理 ${pools.length}`}
          />
          <Button
            type="button"
            size="small"
            variant="outlined"
            disabled={!configReady}
            startIcon={<Plus className="h-3.5 w-3.5" />}
            onClick={onCreate}
          >
            新建策略池
          </Button>
        </div>
      </div>

      {!configReady ? (
        <div className="mt-3 rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2 text-xs text-amber-200">
          {CONFIG_NOT_READY_TEXT}
        </div>
      ) : null}

      {pools.length > 0 ? (
        <div className="mt-3 grid gap-2 md:grid-cols-2">
          {pools.map((pool) => (
            <div
              key={pool.groupRef.name}
              className="rounded-2xl border border-white/10 bg-black/15 p-3"
            >
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold text-text-primary">
                    {pool.groupRef.name}
                  </div>
                  <div className="mt-1 text-xs text-text-secondary">
                    当前出口：{pool.currentProxyName}
                  </div>
                  <div className="mt-1 text-xs text-text-secondary">
                    {pool.memberCount > 0
                      ? `已手动添加 ${pool.memberCount} 个节点`
                      : '还没有手动添加节点'}
                  </div>
                </div>

                <div className="flex shrink-0 flex-wrap items-center gap-2">
                  <Chip
                    size="small"
                    variant="outlined"
                    label={pool.groupRef.displayType}
                  />
                  <Chip
                    size="small"
                    variant="outlined"
                    color={pool.runtimeLoaded ? 'success' : 'default'}
                    label={pool.runtimeLoaded ? '已载入' : '未载入'}
                  />
                  <Button
                    type="button"
                    size="small"
                    variant="outlined"
                    disabled={!configReady}
                    startIcon={<SlidersHorizontal className="h-3.5 w-3.5" />}
                    onClick={() => onEdit(pool.groupRef)}
                  >
                    配置成员
                  </Button>
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="mt-3 rounded-2xl border border-dashed border-white/10 px-3 py-4 text-sm text-text-secondary">
          {EMPTY_TEXT}
        </div>
      )}
    </Box>
  )
}
