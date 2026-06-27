import { AppRuntimePlanningPanel } from '@/components/advanced/app-runtime-planning-panel'
import { BlackholeBreakerPanel } from '@/components/advanced/blackhole-breaker-panel'
import { CoreUpgradePanel } from '@/components/advanced/core-upgrade-panel'
import { EgressIdentityPanel } from '@/components/advanced/egress-identity-panel'
import { EgressMonitorPanel } from '@/components/advanced/egress-monitor-panel'
import { IpReputationPanel } from '@/components/advanced/ip-reputation-panel'
import { LifecycleAuditLogPanel } from '@/components/advanced/lifecycle-audit-log-panel'
import { MultipathConfigPanel } from '@/components/advanced/multipath-config-panel'
import { PerformanceMonitor } from '@/components/advanced/performance-monitor'
import { ResidentialPoolPanel } from '@/components/advanced/residential-pool-panel'
import { SecurityConfigPanel } from '@/components/advanced/security-config-panel'
import { SecurityPolicyPanel } from '@/components/advanced/security-policy-panel'
import { TelemetryDiagnosticsPanel } from '@/components/advanced/telemetry-diagnostics-panel'
import { TimezoneSpoofPanel } from '@/components/advanced/timezone-spoof-panel'
import { IngressCountermeasurePanel } from '@/components/security/ingress-countermeasure-panel'
import { LocalStealthPanel } from '@/components/security/local-stealth-panel'
import { SessionAffinityBindings as SessionAffinityBindingsPanel } from '@/components/security/session-affinity-bindings'
import { SessionAffinityConfig as SessionAffinityConfigPanel } from '@/components/security/session-affinity-config'
import type { AdvancedConfig, CoordinatorStatus } from '@/services/coordinator'

import { ADVANCED_TAB_IDS, type AdvancedTabId } from './constants'
import { AdvancedTabPanel } from './tab-panel'

interface AdvancedTabContentProps {
  activeTab: AdvancedTabId
  config: AdvancedConfig
  status: CoordinatorStatus
  hasUnsavedSecurityPolicies: boolean
  onRefreshStatus: () => Promise<CoordinatorStatus | null>
  onConfigChange: (config: AdvancedConfig) => void
}

export function AdvancedTabContent({
  activeTab,
  config,
  status,
  hasUnsavedSecurityPolicies,
  onRefreshStatus,
  onConfigChange,
}: AdvancedTabContentProps) {
  const updateConfig = <K extends keyof AdvancedConfig>(
    key: K,
    value: AdvancedConfig[K],
  ) => {
    onConfigChange({ ...config, [key]: value })
  }

  return (
    <>
      <AdvancedTabPanel activeTab={activeTab} tabId={ADVANCED_TAB_IDS.security}>
        <div className="space-y-4">
          <SecurityConfigPanel
            config={config.security}
            onChange={(security) => updateConfig('security', security)}
          />
          <IngressCountermeasurePanel
            config={config.ingress_countermeasure}
            onChange={(ingress_countermeasure) =>
              updateConfig('ingress_countermeasure', ingress_countermeasure)
            }
          />
        </div>
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.securityPolicies}
      >
        <SecurityPolicyPanel
          policies={config.security_policies ?? []}
          hasUnsavedChanges={hasUnsavedSecurityPolicies}
          onChange={(security_policies) =>
            updateConfig('security_policies', security_policies)
          }
        />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.localStealth}
      >
        <LocalStealthPanel />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.egressIdentity}
      >
        <EgressIdentityPanel
          config={config.egress_identity}
          status={status}
          onRefreshStatus={onRefreshStatus}
          residentialPool={config.residential_pool}
          onChange={(egress_identity) =>
            updateConfig('egress_identity', egress_identity)
          }
        />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.sessionAffinity}
      >
        <div className="space-y-4">
          <SessionAffinityConfigPanel
            config={config.session_affinity}
            onChange={(session_affinity) =>
              updateConfig('session_affinity', session_affinity)
            }
          />
          <SessionAffinityBindingsPanel
            status={status}
            onRefreshStatus={onRefreshStatus}
          />
        </div>
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.appRuntime}
      >
        <div className="space-y-4">
          <AppRuntimePlanningPanel />
          <CoreUpgradePanel />
          <TelemetryDiagnosticsPanel />
          <LifecycleAuditLogPanel />
        </div>
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.egressMonitor}
      >
        <EgressMonitorPanel
          config={config.egress_monitor}
          onChange={(egress_monitor) =>
            updateConfig('egress_monitor', egress_monitor)
          }
        />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.residentialPool}
      >
        <ResidentialPoolPanel
          config={config.residential_pool}
          onChange={(residential_pool) =>
            updateConfig('residential_pool', residential_pool)
          }
        />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.ipReputation}
      >
        <IpReputationPanel
          config={config.ip_reputation}
          onChange={(ip_reputation) =>
            updateConfig('ip_reputation', ip_reputation)
          }
        />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.blackholeBreaker}
      >
        <BlackholeBreakerPanel
          config={config.blackhole_breaker}
          onChange={(blackhole_breaker) =>
            updateConfig('blackhole_breaker', blackhole_breaker)
          }
        />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.timezoneSpoof}
      >
        <TimezoneSpoofPanel
          config={config.timezone_spoof}
          onChange={(timezone_spoof) =>
            updateConfig('timezone_spoof', timezone_spoof)
          }
        />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.multipath}
      >
        <MultipathConfigPanel
          config={config.multipath}
          onChange={(multipath) => updateConfig('multipath', multipath)}
        />
      </AdvancedTabPanel>

      <AdvancedTabPanel
        activeTab={activeTab}
        tabId={ADVANCED_TAB_IDS.performance}
      >
        <PerformanceMonitor status={status} onRefresh={onRefreshStatus} />
      </AdvancedTabPanel>
    </>
  )
}
