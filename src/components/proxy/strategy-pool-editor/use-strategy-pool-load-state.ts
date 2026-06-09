import { useEffect, useState } from 'react'

import { enhanceProfiles } from '@/services/cmds'

import { resolveLoadWarningMessage } from './load-warning-message'
import { loadEditableStrategyGroup } from './strategy-group-loader'

interface UseStrategyPoolLoadStateOptions {
  open: boolean
  group: IProxyGroupItem | null
  profileUid: string
  groupsProperty: string
  onResetState: () => void
}

export function useStrategyPoolLoadState({
  open,
  group,
  profileUid,
  groupsProperty,
  onResetState,
}: UseStrategyPoolLoadStateOptions) {
  const [loading, setLoading] = useState(false)
  const [loadWarning, setLoadWarning] = useState('')
  const [selectedNames, setSelectedNames] = useState<string[]>([])

  useEffect(() => {
    if (!open || !group) {
      setLoading(false)
      setSelectedNames([])
      setLoadWarning('')
      onResetState()
      return
    }

    let cancelled = false

    setLoading(true)
    setSelectedNames([])
    setLoadWarning(
      resolveLoadWarningMessage(groupsProperty ? [] : ['profileNotReady']),
    )
    onResetState()

    void (async () => {
      let result = await loadEditableStrategyGroup(
        group,
        profileUid,
        groupsProperty,
      )

      if (
        groupsProperty &&
        result.warnings.includes('groupsReadFailed') &&
        (await enhanceProfiles())
      ) {
        result = await loadEditableStrategyGroup(
          group,
          profileUid,
          groupsProperty,
        )
      }

      if (cancelled) return

      setSelectedNames(result.selectedNames)
      setLoadWarning(resolveLoadWarningMessage(result.warnings))
    })()
      .catch(() => {
        if (cancelled) return
        setSelectedNames([])
        setLoadWarning(
          '策略池配置暂时读取失败，先展示全部节点；配置恢复后可以继续保存。',
        )
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [group, groupsProperty, onResetState, open, profileUid])

  return {
    loading,
    loadWarning,
    selectedNames,
    setSelectedNames,
  }
}
