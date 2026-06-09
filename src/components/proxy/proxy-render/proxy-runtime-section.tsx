import { cn } from '@/utils/cn'

import type { IRenderItem } from '../render-list/types'

interface ProxyRuntimeSectionProps {
  item: IRenderItem
}

const SECTION_TONE_CLASS: Record<string, string> = {
  runtime: 'border-teal-500/20 bg-teal-500/5 text-teal-500',
  manual: 'border-sky-500/20 bg-sky-500/5 text-sky-400',
  strategy: 'border-amber-500/20 bg-amber-500/5 text-amber-400',
}

export function ProxyRuntimeSection({ item }: ProxyRuntimeSectionProps) {
  const toneClass =
    SECTION_TONE_CLASS[item.sectionKind || 'manual'] ??
    SECTION_TONE_CLASS.manual
  const isRuntime = item.sectionKind === 'runtime'

  return (
    <div
      className={cn(
        'mx-2 mt-2 rounded-xl border px-3 py-2',
        isRuntime ? 'mb-3' : 'mb-1',
        toneClass,
      )}
    >
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-[11px] font-semibold tracking-[0.18em]">
          {item.sectionTitle}
        </span>
        {item.runtimeObserved === false && (
          <span className="rounded-full border border-gray-500/30 px-2 py-0.5 text-[10px] text-gray-400">
            未观测
          </span>
        )}
      </div>

      {item.runtimePath?.length ? (
        <div className="mt-1 break-all text-sm font-semibold text-white/90">
          {item.runtimePath.join(' -> ')}
        </div>
      ) : null}

      {item.sectionDescription && (
        <div
          className={cn(
            'mt-1 text-xs',
            isRuntime ? 'text-gray-300' : 'text-gray-400',
          )}
        >
          {item.sectionDescription}
        </div>
      )}

      {item.runtimeDescription && (
        <div className="mt-1 text-xs text-gray-400">
          {item.runtimeDescription}
        </div>
      )}
    </div>
  )
}
