import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

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
  /** If provided, only show rules matching this source prefix */
  sourceFilter?: string
}

export const ProfileRulesPanel = (props: Props) => {
  const { sourceFilter } = props
  const { t } = useTranslation()
  const { rules = [] } = useRulesData()
  const { refreshRules, refreshRuleProviders } = useAppRefreshers()
  const [match, setMatch] = useState(() => (_: string) => true)
  const virtuosoRef = useRef<VirtualListHandle>(null)
  const [showScrollTop, setShowScrollTop] = useState(false)
  const pageVisible = useVisibility()

  const conflicts = useMemo(() => detectRuleConflicts(rules), [rules])
  const { errorCount, warningCount, shadowedIndices } = useMemo(
    () => getConflictSummary(conflicts),
    [conflicts],
  )
  const [showConflicts, setShowConflicts] = useState(false)
  const [showCreateRule, setShowCreateRule] = useState(false)

  const hitStats = useMemo(() => {
    let totalHit = 0
    let totalMiss = 0
    let activeCount = 0
    for (const r of rules) {
      if (r.extra?.disabled || r.extra?.deleted) continue
      activeCount++
      totalHit += r.extra?.hitCount ?? 0
      totalMiss += r.extra?.missCount ?? 0
    }
    const total = totalHit + totalMiss
    return { totalHit, totalMiss, activeCount, hitRate: total > 0 ? ((totalHit / total) * 100).toFixed(1) : null }
  }, [rules])

  useEffect(() => {
    refreshRules()
    refreshRuleProviders()
    if (pageVisible) {
      refreshRules()
      refreshRuleProviders()
    }
  }, [refreshRules, refreshRuleProviders, pageVisible])

  const filteredRules = useMemo(() => {
    let result = rules
    // Filter by source if provided
    if (sourceFilter) {
      if (sourceFilter === 'profile') {
        result = result.filter((r) => r.source === 'profile')
      } else {
        result = result.filter((r) => r.source === sourceFilter || r.source.startsWith(`provider:${sourceFilter}`))
      }
    }
    if (showConflicts) {
      return result.filter((item) => shadowedIndices.has(item.index))
    }
    return result.filter((item) => match(item.payload ?? ''))
  }, [rules, match, showConflicts, shadowedIndices, sourceFilter])

  const handleScroll = useCallback((e: Event) => {
    setShowScrollTop((e.target as HTMLElement).scrollTop > 100)
  }, [])

  const scrollToTop = () => {
    virtuosoRef.current?.scrollTo({ top: 0, behavior: 'smooth' })
  }

  return (
    <Box className="flex flex-col h-full">
      {/* Header bar */}
      <Box className="flex items-center gap-1 px-2 py-1 border-b border-divider-light dark:border-divider-dark shrink-0">
        <ProviderButton />
        <Button variant="outlined" size="small" onClick={() => setShowCreateRule(true)}>
          + Rule
        </Button>
        {hitStats.totalHit > 0 && (
          <span className="px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400">
            {hitStats.totalHit} hits{hitStats.hitRate ? ` (${hitStats.hitRate}%)` : ''} / {hitStats.activeCount} rules
          </span>
        )}
        {conflicts.length > 0 && (
          <button
            type="button"
            onClick={() => setShowConflicts(!showConflicts)}
            className={showConflicts
              ? 'px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400'
              : 'px-2 py-0.5 rounded text-xs font-medium bg-yellow-100 text-yellow-600 dark:bg-yellow-900/30 dark:text-yellow-400'}
          >
            {showConflicts ? `冲突 ${errorCount + warningCount}` : `⚠ ${errorCount + warningCount} 冲突`}
          </button>
        )}
        <Box className="flex-1" />
        <BaseSearchBox onSearch={(match) => setMatch(() => match)} />
      </Box>

      {/* Rules list */}
      <Box className="flex-1 min-h-0 p-2">
        <Card className="h-full overflow-hidden">
          {filteredRules && filteredRules.length > 0 ? (
            <VirtualList
              ref={virtuosoRef}
              count={filteredRules.length}
              estimateSize={40}
              renderItem={(i) => (
                <RuleItem value={filteredRules[i]} isShadowed={shadowedIndices.has(filteredRules[i].index)} />
              )}
              style={{ height: '100%' }}
              onScroll={handleScroll}
            />
          ) : (
            <BaseEmpty />
          )}
        </Card>
      </Box>

      <CreateRuleDialog open={showCreateRule} onClose={() => setShowCreateRule(false)} />
    </Box>
  )
}
