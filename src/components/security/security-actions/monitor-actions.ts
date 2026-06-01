import { showNotice } from '@/services/notice-service'
import {
  securityStartMonitor,
  securityStopMonitor,
} from '@/services/security'

interface MonitorActionsState {
  monitorEnabled: boolean
  setMonitorEnabled: (enabled: boolean) => void
}

export function createMonitorActions({
  monitorEnabled,
  setMonitorEnabled,
}: MonitorActionsState) {
  const onToggleMonitor = async () => {
    try {
      if (monitorEnabled) {
        await securityStopMonitor()
        showNotice.success('安全监控已停止')
      } else {
        await securityStartMonitor()
        showNotice.success('安全监控已启动')
      }
      setMonitorEnabled(!monitorEnabled)
    } catch (error) {
      showNotice.error(`操作失败: ${error}`)
    }
  }

  return { onToggleMonitor }
}
