import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  BaseEmpty,
  BasePage,
  BaseSearchBox,
  VirtualList,
  type VirtualListHandle,
} from '@/components/base'
import { ScrollTopButton } from '@/components/layout/scroll-top-button'
import { CreateRuleDialog } from '@/components/rule/create-rule-dialog'
import { ProviderButton } from '@/components/rule/provider-button'
import RuleItem from '@/components/rule/rule-item'
import { Box, Button } from '@/components/tailwind'
import { useVisibility } from '@/hooks/ui'
import { useAppRefreshers, useRulesData } from '@/providers/app-data-context'
import { detectRuleConflicts, getConflictSummary } from '@/utils/rule-conflict'

const RulesPage = () => {
  const { t } = useTranslation()
  const { rules = [] } = useRulesData()
  const { refreshRules, refreshRuleProviders } = useAppRefreshers()
  const [match, setMatch] = useState(() => (_: string) => true)
  const virtuosoRef = useRef<VirtualListHandle>(null)
  const [showScrollTop, setShowScrollTop] = useState(false)
  const pageVisible = useVisibility()

  // Rule conflict detection
  const conflicts = useMemo(() => detectRuleConflicts(rules), [rules])
  const { errorCount, warningCount, shadowedIndices } = useMemo(
    () => getConflictSummary(conflicts),
    [conflicts],
  )
  const [showConflicts, setShowConflicts] = useState(false)
  const [showCreateRule, setShowCreateRule] = useState(false)

  // Aggregate hit statistics
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

  // 在组件挂载时和页面获得焦点时刷新规则数据
  useEffect(() => {
    refreshRules()
    refreshRuleProviders()

    if (pageVisible) {
      refreshRules()
      refreshRuleProviders()
    }
  }, [refreshRules, refreshRuleProviders, pageVisible])

  const filteredRules = useMemo(() => {
    if (showConflicts) {
      return rules.filter((item) => shadowedIndices.has(item.index))
    }
    return rules.filter((item) => match(item.payload ?? ''))
  }, [rules, match, showConflicts, shadowedIndices])

  const handleScroll = useCallback((e: Event) => {
    setShowScrollTop((e.target as HTMLElement).scrollTop > 100)
  }, [])

  const scrollToTop = () => {
    virtuosoRef.current?.scrollTo({ top: 0, behavior: 'smooth' })
  }

  return (
    <BasePage
      full
      title={t('rules.page.title')}
      contentStyle={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'auto',
      }}
      header={
        <Box className="flex items-center gap-1">
          <ProviderButton />
          <Button variant="outlined" size="small" onClick={() => setShowCreateRule(true)}>+ Rule</Button>
          {hitStats.totalHit > 0 && (
            <span className="px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400">
              {hitStats.totalHit} hits{hitStats.hitRate ? ` (${hitStats.hitRate}%)` : ''} / {hitStats.activeCount} rules
            </span>
          )}
          {conflicts.length > 0 && (
            <button
              type="button"
              onClick={() => setShowConflicts(!showConflicts)}
              className={showConflicts ? 'px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400' : 'px-2 py-0.5 rounded text-xs font-medium bg-yellow-100 text-yellow-600 dark:bg-yellow-900/30 dark:text-yellow-400'}
            >
              {showConflicts ? `冲突 ${errorCount + warningCount}` : `⚠ ${errorCount + warningCount} 冲突`}
            </button>
          )}
        </Box>
      }
    >
      <Box
        className="pt-4 mb-2 mx-[10px] h-[36px] flex items-center"
      >
        <BaseSearchBox onSearch={(match) => setMatch(() => match)} />
      </Box>

      {filteredRules && filteredRules.length > 0 ? (
        <>
          <VirtualList
            ref={virtuosoRef}
            count={filteredRules.length}
            estimateSize={40}
            renderItem={(i) => <RuleItem value={filteredRules[i]} isShadowed={shadowedIndices.has(filteredRules[i].index)} />}
            style={{ flex: 1 }}
            onScroll={handleScroll}
          />
          <ScrollTopButton onClick={scrollToTop} show={showScrollTop} />
        </>
      ) : (
        <BaseEmpty />
      )}

      <CreateRuleDialog open={showCreateRule} onClose={() => setShowCreateRule(false)} />
    </BasePage>
  )
}

export default RulesPage
