import { useCallback, useState } from 'react'

import { StrategyPoolEditorDialog } from '@/components/proxy/strategy-pool-editor-dialog'
import {
  MANAGED_STRATEGY_POOLS_QUERY_KEY,
  useManagedStrategyPools,
} from '@/components/proxy/strategy-pools/strategy-pool-data'
import { StrategyPoolsSection } from '@/components/proxy/strategy-pools/strategy-pools-section'
import type { StrategyPoolGroupRef } from '@/components/proxy/strategy-pools/types'
import { useAppRefreshers } from '@/providers/app-data-context'
import { queryClient } from '@/services/query-client'

export const ProxyPageStrategyPools = () => {
  const [editingGroup, setEditingGroup] =
    useState<StrategyPoolGroupRef | null>(null)

  const { refreshProxy } = useAppRefreshers()
  const { createStrategyPoolCandidate, groupsOverridePath, pools } =
    useManagedStrategyPools()

  const handleSaved = useCallback(async () => {
    await Promise.all([
      refreshProxy(),
      queryClient.invalidateQueries({
        queryKey: [MANAGED_STRATEGY_POOLS_QUERY_KEY],
      }),
    ])
  }, [refreshProxy])

  return (
    <>
      <StrategyPoolsSection
        configReady={Boolean(groupsOverridePath)}
        pools={pools}
        onCreate={() => setEditingGroup(createStrategyPoolCandidate)}
        onEdit={setEditingGroup}
      />

      <StrategyPoolEditorDialog
        open={Boolean(editingGroup)}
        group={editingGroup}
        onClose={() => setEditingGroup(null)}
        onSaved={handleSaved}
      />
    </>
  )
}
