import { showNotice } from '@/services/notice-service'
import {
  securityCheckDecoyAccess,
  securityCleanupDecoy,
  securityDeployDecoy,
} from '@/services/security'

interface DecoyActionsState {
  decoyPath: string
}

export function createDecoyActions({ decoyPath }: DecoyActionsState) {
  const onDeployDecoy = async () => {
    try {
      await securityDeployDecoy(decoyPath)
      showNotice.success('假配置文件已部署')
    } catch (error) {
      showNotice.error(`部署失败: ${error}`)
    }
  }

  const onCleanupDecoy = async () => {
    try {
      await securityCleanupDecoy(decoyPath)
      showNotice.success('假配置文件已清除')
    } catch (error) {
      showNotice.error(`清除失败: ${error}`)
    }
  }

  const onCheckDecoyAccess = async () => {
    try {
      const accessed = await securityCheckDecoyAccess(decoyPath)
      if (accessed) {
        showNotice.error('假配置文件被访问！')
      } else {
        showNotice.success('假配置文件未被访问')
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
