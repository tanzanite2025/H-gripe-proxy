import { AlertTriangle, Shield } from 'lucide-react'

import type { HoneypotDecoy, NewHoneypotDecoyInput } from './security-honeypot-decoys'
import type { HoneypotDecoyStrategyProfile } from './security-honeypot-decoy-strategy'
import type { SecurityStatus } from '@/services/security'

import HoneypotDecoyPanel from './security-panels/honeypot-decoy-panel'
import SecureStoragePanel from './security-panels/secure-storage-panel'
import SecurityStatusPanel from './security-panels/security-status-panel'
import SelfDestructPanel from './security-panels/self-destruct-panel'

interface SecurityMonitorUIProps {
  monitorEnabled: boolean
  status: SecurityStatus
  honeypotDecoys: HoneypotDecoy[]
  activeDecoyId: string
  decoyPath: string
  encryptionKey: string
  hasEncryptionKey: boolean
  selfDestructConfirm: string
  onToggleMonitor: () => void
  onDecoyPathChange: (path: string) => void
  onActiveDecoyChange: (decoyId: string) => void
  onAddHoneypotDecoy: (input: NewHoneypotDecoyInput) => void
  onRemoveHoneypotDecoy: (decoyId: string) => void
  onHoneypotDecoyEnabledChange: (decoyId: string, enabled: boolean) => void
  onApplyHoneypotDecoyStrategy: (
    profile?: Partial<HoneypotDecoyStrategyProfile>,
  ) => void
  onDeployDecoy: () => void
  onCleanupDecoy: () => void
  onCheckDecoyAccess: () => void
  onGenerateKey: () => void
  onCopyKey: () => void
  onSelfDestructConfirmChange: (value: string) => void
  onSelfDestruct: () => void
}

export default function SecurityMonitorUI({
  monitorEnabled,
  status,
  honeypotDecoys,
  activeDecoyId,
  decoyPath,
  encryptionKey,
  hasEncryptionKey,
  selfDestructConfirm,
  onToggleMonitor,
  onDecoyPathChange,
  onActiveDecoyChange,
  onAddHoneypotDecoy,
  onRemoveHoneypotDecoy,
  onHoneypotDecoyEnabledChange,
  onApplyHoneypotDecoyStrategy,
  onDeployDecoy,
  onCleanupDecoy,
  onCheckDecoyAccess,
  onGenerateKey,
  onCopyKey,
  onSelfDestructConfirmChange,
  onSelfDestruct,
}: SecurityMonitorUIProps) {
  return (
    <div className="p-6">
      <div className="space-y-6">
        <div className="flex items-center gap-2">
          <Shield className="w-5 h-5 text-primary" />
          <h2 className="text-xl font-semibold">
            内生欺骗陷阱（Canary Honeytoken）
          </h2>
        </div>

        <div className="p-4 bg-red-500 text-white rounded-lg">
          <div className="flex items-start gap-2">
            <AlertTriangle className="w-5 h-5 shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-sm">防御究极体</p>
              <p className="text-xs opacity-90 mt-1">
                反调试、内存蜜罐、配置欺骗、自毁机制 -
                全方位防范本地流氓软件扫描和物理攻破
              </p>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <SecurityStatusPanel
            monitorEnabled={monitorEnabled}
            status={status}
            onToggleMonitor={onToggleMonitor}
          />
          <HoneypotDecoyPanel
            honeypotDecoys={honeypotDecoys}
            activeDecoyId={activeDecoyId}
            decoyPath={decoyPath}
            onDecoyPathChange={onDecoyPathChange}
            onActiveDecoyChange={onActiveDecoyChange}
            onAddHoneypotDecoy={onAddHoneypotDecoy}
            onRemoveHoneypotDecoy={onRemoveHoneypotDecoy}
            onHoneypotDecoyEnabledChange={onHoneypotDecoyEnabledChange}
            onApplyHoneypotDecoyStrategy={onApplyHoneypotDecoyStrategy}
            onDeployDecoy={onDeployDecoy}
            onCleanupDecoy={onCleanupDecoy}
            onCheckDecoyAccess={onCheckDecoyAccess}
          />
          <SecureStoragePanel
            encryptionKey={encryptionKey}
            hasEncryptionKey={hasEncryptionKey}
            onGenerateKey={onGenerateKey}
            onCopyKey={onCopyKey}
          />
          <SelfDestructPanel
            selfDestructConfirm={selfDestructConfirm}
            onSelfDestructConfirmChange={onSelfDestructConfirmChange}
            onSelfDestruct={onSelfDestruct}
          />
        </div>
      </div>
    </div>
  )
}
