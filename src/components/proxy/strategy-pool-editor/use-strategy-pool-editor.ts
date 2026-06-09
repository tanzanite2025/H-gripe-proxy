import { useCallback, useMemo, useState } from 'react'

import { useProfiles } from '@/hooks/data'
import { useProxiesData } from '@/providers/app-data-context'

import { buildCandidateOptions } from './build-candidate-options'
import type { UseStrategyPoolEditorOptions } from './types'
import { useStrategyPoolLoadState } from './use-strategy-pool-load-state'
import { useStrategyPoolSave } from './use-strategy-pool-save'

export function useStrategyPoolEditor(options: UseStrategyPoolEditorOptions) {
  const { open, group, onClose, onSaved } = options
  const { current } = useProfiles()
  const { proxies: proxiesData } = useProxiesData()

  const [searchText, setSearchText] = useState('')

  const groupsProperty = current?.option?.groups?.trim() || ''
  const handleResetState = useCallback(() => {
    setSearchText('')
  }, [])

  const {
    loading,
    loadWarning,
    selectedNames,
    setSelectedNames,
  } = useStrategyPoolLoadState({
    open,
    group,
    groupsProperty,
    onResetState: handleResetState,
  })

  const { saving, handleSave } = useStrategyPoolSave({
    group,
    groupsProperty,
    selectedNames,
    onClose,
    onSaved,
  })

  const candidateOptions = useMemo(() => {
    return buildCandidateOptions({
      records: (proxiesData?.records || {}) as Record<string, IProxyItem>,
      selectedNames,
      searchText,
    })
  }, [proxiesData?.records, searchText, selectedNames])

  const toggleSelected = useCallback((name: string, checked?: boolean) => {
    setSelectedNames((prev) => {
      const exists = prev.includes(name)
      const nextChecked = checked ?? !exists

      if (nextChecked) {
        return exists ? prev : [...prev, name]
      }

      return prev.filter((item) => item !== name)
    })
  }, [])

  return {
    candidateOptions,
    canSave:
      !loading &&
      !saving &&
      selectedNames.length > 0 &&
      Boolean(groupsProperty),
    handleSave,
    loadWarning,
    loading,
    saving,
    searchText,
    selectedNames,
    selectedNameSet: new Set(selectedNames),
    setSearchText,
    toggleSelected,
  }
}
