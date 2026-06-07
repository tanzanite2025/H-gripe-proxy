import type { RefObject } from 'react'

import { debugLog } from '@/utils/misc'

export const debugProfileSwitch = (
  action: string,
  profile: string,
  extra?: unknown,
) => {
  const timestamp = new Date().toISOString().substring(11, 23)
  debugLog(`[Profile-Debug][${timestamp}] ${action}: ${profile}`, extra || '')
}

export const isRequestOutdated = (
  currentSequence: number,
  requestSequenceRef: RefObject<number>,
  profile: string,
) => {
  if (currentSequence !== requestSequenceRef.current) {
    debugProfileSwitch(
      'REQUEST_OUTDATED',
      profile,
      `current=${currentSequence}, latest=${requestSequenceRef.current}`,
    )
    return true
  }

  return false
}

export const isOperationAborted = (
  abortController: AbortController,
  profile: string,
) => {
  if (abortController.signal.aborted) {
    debugProfileSwitch('OPERATION_ABORTED', profile)
    return true
  }

  return false
}

export type ProfileSelectionState = 'none' | 'all' | 'partial'
