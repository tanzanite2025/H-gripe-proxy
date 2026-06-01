import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'

import { showNotice } from '@/services/notice-service'
import {
  type SecurityStatus,
  securityCheckEncryptionKey,
  securityCheckStatus,
} from '@/services/security'

import {
  DEFAULT_HONEYPOT_DECOY_ID,
  type NewHoneypotDecoyInput,
  addHoneypotDecoy,
  createDefaultHoneypotDecoys,
  getActiveHoneypotDecoyPath,
  getEnabledHoneypotDecoys,
  normalizeActiveHoneypotDecoyId,
  removeHoneypotDecoy,
  selectActiveHoneypotDecoyId,
  setHoneypotDecoyEnabled,
  updateActiveHoneypotDecoyPath,
} from './security-honeypot-decoys'
import {
  type HoneypotDecoyStrategyProfile,
  createHoneypotDecoyStrategyProfile,
  mergeHoneypotDecoyStrategy,
} from './security-honeypot-decoy-strategy'
import { createSecurityMonitorActions } from './security-monitor-actions'

const DEFAULT_SECURITY_STATUS: SecurityStatus = {
  compromised: false,
  debugger_present: false,
  memory_scanning: false,
  leak_detected: false,
  leak_type: null,
}

export function useSecurityMonitorController() {
  const [monitorEnabled, setMonitorEnabled] = useState(false)
  const [status, setStatus] = useState<SecurityStatus>(DEFAULT_SECURITY_STATUS)
  const [honeypotDecoys, setHoneypotDecoys] = useState(createDefaultHoneypotDecoys)
  const [activeDecoyId, setActiveDecoyId] = useState(DEFAULT_HONEYPOT_DECOY_ID)
  const [encryptionKey, setEncryptionKey] = useState('')
  const [hasEncryptionKey, setHasEncryptionKey] = useState(false)
  const [selfDestructConfirm, setSelfDestructConfirm] = useState('')
  const decoyPath = getActiveHoneypotDecoyPath(honeypotDecoys, activeDecoyId)
  const enabledDecoys = getEnabledHoneypotDecoys(honeypotDecoys)

  useEffect(() => {
    const unlisten = listen<SecurityStatus>('security-alert', (event) => {
      const newStatus = event.payload
      setStatus(newStatus)

      if (newStatus.compromised) {
        showNotice.error('安全状态已被破坏！')
      }
      if (newStatus.leak_detected) {
        showNotice.error(`泄漏检测: ${newStatus.leak_type ?? '未知类型'}`)
      }
    })

    return () => {
      void unlisten.then((fn) => fn())
    }
  }, [])

  useEffect(() => {
    securityCheckStatus()
      .then((newStatus) => setStatus(newStatus))
      .catch((error) => console.error('检查安全状态失败', error))
  }, [])

  useEffect(() => {
    checkEncryptionKey()
  }, [])

  const checkEncryptionKey = async () => {
    try {
      const hasKey = await securityCheckEncryptionKey()
      setHasEncryptionKey(hasKey)
    } catch (error) {
      console.error('检查加密密钥失败', error)
    }
  }

  const actions = createSecurityMonitorActions({
    monitorEnabled,
    decoyPath,
    enabledDecoys,
    encryptionKey,
    selfDestructConfirm,
    setMonitorEnabled,
    setEncryptionKey,
  })

  const handleDecoyPathChange = (path: string) => {
    setHoneypotDecoys((decoys) =>
      updateActiveHoneypotDecoyPath(decoys, activeDecoyId, path),
    )
  }

  const handleActiveDecoyChange = (decoyId: string) => {
    setActiveDecoyId((currentDecoyId) =>
      selectActiveHoneypotDecoyId(honeypotDecoys, decoyId, currentDecoyId),
    )
  }

  const handleAddHoneypotDecoy = (input: NewHoneypotDecoyInput) => {
    setHoneypotDecoys((decoys) => {
      const nextDecoys = addHoneypotDecoy(decoys, input)
      const nextDecoyId = nextDecoys[nextDecoys.length - 1]?.id

      if (nextDecoyId) {
        setActiveDecoyId(nextDecoyId)
      }

      return nextDecoys
    })
  }

  const handleRemoveHoneypotDecoy = (decoyId: string) => {
    setHoneypotDecoys((decoys) => {
      const nextDecoys = removeHoneypotDecoy(decoys, decoyId)

      setActiveDecoyId((currentDecoyId) =>
        selectActiveHoneypotDecoyId(nextDecoys, currentDecoyId),
      )

      return nextDecoys
    })
  }

  const handleHoneypotDecoyEnabledChange = (
    decoyId: string,
    enabled: boolean,
  ) => {
    setHoneypotDecoys((decoys) => {
      const nextDecoys = setHoneypotDecoyEnabled(decoys, decoyId, enabled)

      setActiveDecoyId((currentDecoyId) => {
        const currentDecoy = nextDecoys.find((decoy) => decoy.id === currentDecoyId)
        if (currentDecoy?.enabled) {
          return currentDecoyId
        }

        const enabledDecoys = getEnabledHoneypotDecoys(nextDecoys)
        return selectActiveHoneypotDecoyId(
          enabledDecoys.length > 0 ? enabledDecoys : nextDecoys,
          currentDecoyId,
        )
      })

      return nextDecoys
    })
  }

  const handleApplyHoneypotDecoyStrategy = (
    profile?: Partial<HoneypotDecoyStrategyProfile>,
  ) => {
    setHoneypotDecoys((decoys) => {
      const nextDecoys = mergeHoneypotDecoyStrategy(
        decoys,
        createHoneypotDecoyStrategyProfile(profile),
      )

      setActiveDecoyId((currentDecoyId) =>
        normalizeActiveHoneypotDecoyId(nextDecoys, currentDecoyId),
      )

      return nextDecoys
    })
  }

  return {
    monitorEnabled,
    status,
    honeypotDecoys,
    activeDecoyId,
    decoyPath,
    encryptionKey,
    hasEncryptionKey,
    selfDestructConfirm,
    ...actions,
    onDecoyPathChange: handleDecoyPathChange,
    onActiveDecoyChange: handleActiveDecoyChange,
    onAddHoneypotDecoy: handleAddHoneypotDecoy,
    onRemoveHoneypotDecoy: handleRemoveHoneypotDecoy,
    onHoneypotDecoyEnabledChange: handleHoneypotDecoyEnabledChange,
    onApplyHoneypotDecoyStrategy: handleApplyHoneypotDecoyStrategy,
    onSelfDestructConfirmChange: setSelfDestructConfirm,
  }
}
