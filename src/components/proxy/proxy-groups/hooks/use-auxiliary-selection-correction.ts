import { useEffect, useRef } from 'react'

import {
  isAuxiliarySelectionName,
  isHiddenProxyName,
  pickPreferredProxyNameFromGroup,
} from '@/services/proxy-display'
import type { IProxyGroupItem } from '@/types/proxy'
interface SelectionCorrection {
  groupName: string
  previousProxy: string
  proxyName: string
}

const buildSelectionCorrections = (proxiesData: any): SelectionCorrection[] => {
  if (!proxiesData?.records) {
    return []
  }

  const groups = [proxiesData.global, ...(proxiesData.groups || [])].filter(
    Boolean,
  ) as IProxyGroupItem[]

  return groups
    .map((group) => {
      if (group.type !== 'Selector') {
        return null
      }

      const currentName = group.now?.trim() || ''
      if (
        !currentName ||
        (!isAuxiliarySelectionName(currentName, proxiesData.records) &&
          !isHiddenProxyName(currentName))
      ) {
        return null
      }

      const targetName = pickPreferredProxyNameFromGroup(
        group,
        proxiesData.records,
        group.now,
      )

      if (!targetName || targetName === currentName) {
        return null
      }

      return {
        groupName: group.name,
        previousProxy: currentName,
        proxyName: targetName,
      }
    })
    .filter((correction): correction is SelectionCorrection => Boolean(correction))
}

export function useAuxiliarySelectionCorrection(
  proxiesData: any,
  changeProxy: (
    groupName: string,
    proxyName: string,
    previousProxy?: string,
  ) => void,
) {
  const correctionSignatureRef = useRef('')

  useEffect(() => {
    const corrections = buildSelectionCorrections(proxiesData)

    if (corrections.length === 0) {
      correctionSignatureRef.current = ''
      return
    }

    const signature = corrections
      .map(
        ({ groupName, previousProxy, proxyName }) =>
          `${groupName}:${previousProxy}->${proxyName}`,
      )
      .join('|')

    if (correctionSignatureRef.current === signature) {
      return
    }

    correctionSignatureRef.current = signature
    corrections.forEach(({ groupName, previousProxy, proxyName }) => {
      changeProxy(groupName, proxyName, previousProxy)
    })
  }, [changeProxy, proxiesData])
}
