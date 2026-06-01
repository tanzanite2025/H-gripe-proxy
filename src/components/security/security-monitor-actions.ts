import { createDecoyActions } from './security-actions/decoy-actions'
import { createKeyActions } from './security-actions/key-actions'
import { createMonitorActions } from './security-actions/monitor-actions'
import { createSelfDestructActions } from './security-actions/self-destruct-actions'

interface SecurityMonitorActionState {
  monitorEnabled: boolean
  decoyPath: string
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
