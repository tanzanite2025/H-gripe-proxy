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
  createDefaultHoneypotDecoys,
  getActiveHoneypotDecoyPath,
  updateActiveHoneypotDecoyPath,
} from './security-honeypot-decoys'
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

  return {
    monitorEnabled,
    status,
    decoyPath,
    encryptionKey,
    hasEncryptionKey,
    selfDestructConfirm,
    ...actions,
    onDecoyPathChange: handleDecoyPathChange,
    onSelfDestructConfirmChange: setSelfDestructConfirm,
  }
}
