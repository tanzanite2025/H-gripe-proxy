import { createDecoyActions } from './security-actions/decoy-actions'
import { createKeyActions } from './security-actions/key-actions'
import { createMonitorActions } from './security-actions/monitor-actions'
import { createSelfDestructActions } from './security-actions/self-destruct-actions'
import type { HoneypotDecoy } from './security-honeypot-decoys'

interface SecurityMonitorActionState {
  monitorEnabled: boolean
  decoyPath: string
  enabledDecoys: HoneypotDecoy[]
  encryptionKey: string
  selfDestructConfirm: string
  setMonitorEnabled: (enabled: boolean) => void
  setEncryptionKey: (key: string) => void
}

export function createSecurityMonitorActions(state: SecurityMonitorActionState) {
  return {
    ...createMonitorActions({
      monitorEnabled: state.monitorEnabled,
      setMonitorEnabled: state.setMonitorEnabled,
    }),
    ...createDecoyActions({
      decoyPath: state.decoyPath,
      enabledDecoys: state.enabledDecoys,
    }),
    ...createKeyActions({
      encryptionKey: state.encryptionKey,
      setEncryptionKey: state.setEncryptionKey,
    }),
    ...createSelfDestructActions({
      selfDestructConfirm: state.selfDestructConfirm,
    }),
  }
}
