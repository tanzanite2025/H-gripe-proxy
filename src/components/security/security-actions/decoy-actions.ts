import { showNotice } from '@/services/notice-service'
import {
  securityCheckDecoyPlanAccess,
  securityCleanupDecoyPlan,
  securityDeployDecoyPlan,
} from '@/services/security'

import type { HoneypotDecoy } from '../security-honeypot-decoys'

interface DecoyActionsState {
  decoyPath: string
  enabledDecoys: HoneypotDecoy[]
}

export function createDecoyActions({
  decoyPath,
  enabledDecoys,
}: DecoyActionsState) {
  const getDeploymentPlan = () => {
    const paths = enabledDecoys
      .map((decoy) => decoy.path)
      .filter((path): path is string => path.trim().length > 0)

    return { paths: paths.length > 0 ? paths : [decoyPath] }
  }

  const onDeployDecoy = async () => {
    try {
      const result = await securityDeployDecoyPlan(getDeploymentPlan())
      showNotice.success(`假配置文件已部署: ${result.succeeded}/${result.total}`)
    } catch (error) {
      showNotice.error(`部署失败: ${error}`)
    }
  }

  const onCleanupDecoy = async () => {
    try {
      const result = await securityCleanupDecoyPlan(getDeploymentPlan())
      showNotice.success(`假配置文件已清除: ${result.succeeded}/${result.total}`)
    } catch (error) {
      showNotice.error(`清除失败: ${error}`)
    }
  }

  const onCheckDecoyAccess = async () => {
    try {
      const result = await securityCheckDecoyPlanAccess(getDeploymentPlan())
      const accessedCount = result.accessed.filter((item) => item.accessed).length

      if (accessedCount > 0) {
        showNotice.error(`假配置文件被访问: ${accessedCount}`)
      } else {
        showNotice.success(`假配置文件未被访问: ${result.total}`)
      }
    } catch (error) {
      showNotice.error(`检查失败: ${error}`)
    }
  }

  return {
    onDeployDecoy,
    onCleanupDecoy,
    onCheckDecoyAccess,
  }
}
