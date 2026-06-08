import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import {
  BaseEmpty,
  BaseSearchBox,
  VirtualList,
  type VirtualListHandle,
} from '@/components/base'
import { CreateRuleDialog } from '@/components/rule/create-rule-dialog'
import { ProviderButton } from '@/components/rule/provider-button'
import RuleItem from '@/components/rule/rule-item'
import { Box, Button, Card } from '@/components/tailwind'
import { useVisibility } from '@/hooks/ui'
import { useAppRefreshers, useRulesData } from '@/providers/app-data-context'
import { detectRuleConflicts, getConflictSummary } from '@/utils/rule-conflict'

interface Props {
  sourceFilter?: string
}

export const ProfileRulesPanel = ({ sourceFilter }: Props) => {
  const { rules = [] } = useRulesData()
  const { refreshRules, refreshRuleProviders } = useAppRefreshers()
  const pageVisible = useVisibility()
  const virtuosoRef = useRef<VirtualListHandle>(null)

  const [match, setMatch] = useState(() => (_: string) => true)
  const [showConflicts, setShowConflicts] = useState(false)
  const [showCreateRule, setShowCreateRule] = useState(false)

  const conflicts = useMemo(() => detectRuleConflicts(rules), [rules])
  const { errorCount, warningCount, shadowedIndices } = useMemo(
    () => getConflictSummary(conflicts),
    [conflicts],
  )

  const hitStats = useMemo(() => {
    let totalHit = 0
    let totalMiss = 0
    let activeCount = 0

    for (const rule of rules) {
      if (rule.extra?.disabled || rule.extra?.deleted) continue

      activeCount += 1
      totalHit += rule.extra?.hitCount ?? 0
      totalMiss += rule.extra?.missCount ?? 0
    }

    const total = totalHit + totalMiss

    return {
      totalHit,
      activeCount,
      hitRate: total > 0 ? ((totalHit / total) * 100).toFixed(1) : null,
    }
  }, [rules])

  useEffect(() => {
    refreshRules()
    refreshRuleProviders()

    if (pageVisible) {
      refreshRules()
      refreshRuleProviders()
    }
  }, [pageVisible, refreshRuleProviders, refreshRules])

  const filteredRules = useMemo(() => {
    let result = rules

    if (sourceFilter) {
      if (sourceFilter === 'profile') {
        result = result.filter((rule) => rule.source === 'profile')
      } else {
        result = result.filter(
          (rule) =>
            rule.source === sourceFilter ||
            rule.source.startsWith(`provider:${sourceFilter}`),
        )
      }
    }

    if (showConflicts) {
      return result.filter((rule) => shadowedIndices.has(rule.index))
    }

    return result.filter((rule) => match(rule.payload ?? ''))
  }, [match, rules, shadowedIndices, showConflicts, sourceFilter])

  const handleScroll = useCallback((event: Event) => {
    const target = event.target as HTMLElement | null
    if (!target) return
  }, [])

  const conflictCount = errorCount + warningCount

  return (
    <Box className="flex h-full flex-col">
      <Box className="shrink-0 border-b border-divider-light px-2 py-1 dark:border-divider-dark">
        <Box className="flex items-center gap-1">
          <ProviderButton />

          <Button
            variant="outlined"
            size="small"
            onClick={() => setShowCreateRule(true)}
          >
            + Rule
          </Button>

          {hitStats.totalHit > 0 && (
            <span className="rounded bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-600 dark:bg-blue-900/30 dark:text-blue-400">
              {hitStats.totalHit} hits
              {hitStats.hitRate ? ` (${hitStats.hitRate}%)` : ''} /{' '}
              {hitStats.activeCount} rules
            </span>
          )}

          {conflictCount > 0 && (
            <button
              type="button"
              onClick={() => setShowConflicts((current) => !current)}
              className={
                showConflicts
                  ? 'rounded bg-red-100 px-2 py-0.5 text-xs font-medium text-red-600 dark:bg-red-900/30 dark:text-red-400'
                  : 'rounded bg-yellow-100 px-2 py-0.5 text-xs font-medium text-yellow-600 dark:bg-yellow-900/30 dark:text-yellow-400'
              }
            >
              {showConflicts
                ? `冲突 ${conflictCount}`
                : `显示冲突 ${conflictCount}`}
            </button>
          )}

          <Box className="flex-1" />
          <BaseSearchBox onSearch={(nextMatch) => setMatch(() => nextMatch)} />
        </Box>
      </Box>

      <Box className="min-h-0 flex-1 p-2">
        <Card className="h-full overflow-hidden">
          {filteredRules.length > 0 ? (
            <VirtualList
              ref={virtuosoRef}
              count={filteredRules.length}
              estimateSize={40}
              renderItem={(index) => (
                <RuleItem
                  value={filteredRules[index]}
                  isShadowed={shadowedIndices.has(filteredRules[index].index)}
                />
              )}
              style={{ height: '100%' }}
              onScroll={handleScroll}
            />
          ) : (
            <BaseEmpty />
          )}
        </Card>
      </Box>

      <CreateRuleDialog
        open={showCreateRule}
        onClose={() => setShowCreateRule(false)}
      />
    </Box>
  )
}
