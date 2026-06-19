import {
  categorizeProxyGroup,
  isBuiltinPolicyName,
  isHiddenProxyName,
  isProxyGroupItem,
} from '@/services/proxy-display'
import type { IProxyItem } from '@/types/proxy'

import type { CandidateOption } from './types'

interface BuildCandidateOptionsOptions {
  records: Record<string, IProxyItem>
  selectedNames: string[]
  searchText: string
}

export const buildCandidateOptions = ({
  records,
  selectedNames,
  searchText,
}: BuildCandidateOptionsOptions) => {
  const selectedOrder = new Map(
    selectedNames.map((name, index) => [name, index]),
  )
  const options = new Map<string, CandidateOption>()

  ;(Object.values(records) as IProxyItem[]).forEach((record) => {
    if (!record?.name) return
    if (isBuiltinPolicyName(record.name)) return
    if (isHiddenProxyName(record.name)) return
    if (categorizeProxyGroup(record) === 'auxiliary') return
    if (isProxyGroupItem(record)) return

    options.set(record.name, {
      name: record.name,
      type: record.type,
      provider: record.provider,
      isGroup: false,
    })
  })

  selectedNames.forEach((name) => {
    if (isHiddenProxyName(name) || isBuiltinPolicyName(name)) return
    if (options.has(name)) return

    const record = records[name]
    if (!record) {
      options.set(name, {
        name,
        type: 'Unknown',
        isGroup: false,
      })
      return
    }

    options.set(name, {
      name,
      type: record.type,
      provider: record.provider,
      isGroup: isProxyGroupItem(record),
    })
  })

  const keyword = searchText.trim().toLowerCase()

  return Array.from(options.values())
    .filter((option) => {
      if (!keyword) return true

      return [option.name, option.provider, option.type]
        .filter(Boolean)
        .some((value) => value!.toLowerCase().includes(keyword))
    })
    .sort((left, right) => {
      const leftIndex = selectedOrder.get(left.name)
      const rightIndex = selectedOrder.get(right.name)

      if (leftIndex != null && rightIndex != null) {
        return leftIndex - rightIndex
      }

      if (leftIndex != null) return -1
      if (rightIndex != null) return 1

      return left.name.localeCompare(right.name, 'zh-CN', {
        numeric: true,
        sensitivity: 'base',
      })
    })
}
