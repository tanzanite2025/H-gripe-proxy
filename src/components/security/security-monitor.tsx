import SecurityMonitorUI from './security-monitor-ui'
import { useSecurityMonitorController } from './use-security-monitor-controller'

export default function SecurityMonitor() {
  const controller = useSecurityMonitorController()

  return <SecurityMonitorUI {...controller} />
}
