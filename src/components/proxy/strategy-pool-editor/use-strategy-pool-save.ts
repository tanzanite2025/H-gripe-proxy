import { useLockFn } from 'ahooks'
import { useState } from 'react'

import { useProfiles } from '@/hooks/data'
import { saveProfileFile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import type { StrategyPoolGroupRef } from '../strategy-pools/types'

import { buildStrategyGroupYaml, cloneGroupConfig } from './group-config'
import { loadEditableStrategyGroup } from './strategy-group-loader'

interface UseStrategyPoolSaveOptions {
  group: StrategyPoolGroupRef | null
  groupsProperty: string
  selectedNames: string[]
  onClose: () => void
  onSaved?: () => Promise<void> | void
}

export function useStrategyPoolSave({
  group,
  groupsProperty,
  selectedNames,
  onClose,
  onSaved,
}: UseStrategyPoolSaveOptions) {
  const { mutateProfiles } = useProfiles()
  const [saving, setSaving] = useState(false)

  const handleSave = useLockFn(async () => {
    if (!group) return

    if (!groupsProperty) {
      showNotice.error(
        '当前策略池覆写配置还没准备好，暂时无法保存，请稍后再试。',
      )
      return
    }

    if (selectedNames.length === 0) {
      showNotice.error('策略池至少要保留一个成员。')
      return
    }

    setSaving(true)

    try {
      const result = await loadEditableStrategyGroup(group, groupsProperty)
      const nextGroup = cloneGroupConfig(result.state.baseGroup)

      nextGroup.proxies = [...selectedNames]
      delete nextGroup.use
      delete nextGroup['include-all']
      delete nextGroup['include-all-proxies']
      delete nextGroup['include-all-providers']
      delete nextGroup.filter
      delete nextGroup['exclude-filter']
      delete nextGroup['exclude-type']

      const nextYaml = buildStrategyGroupYaml(
        group.name,
        result.sequence,
        nextGroup,
      )

      if (!(await saveProfileFile(groupsProperty, nextYaml))) {
        throw new Error('策略池成员保存失败。')
      }

      await Promise.all([mutateProfiles(), onSaved?.()])
      showNotice.success('shared.feedback.notifications.saved')
      onClose()
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSaving(false)
    }
  })

  return {
    handleSave,
    saving,
  }
}
