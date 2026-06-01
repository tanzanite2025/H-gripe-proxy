import { showNotice } from '@/services/notice-service'
import { securitySelfDestruct } from '@/services/security'

interface SelfDestructActionsState {
  selfDestructConfirm: string
}

export function createSelfDestructActions({
  selfDestructConfirm,
}: SelfDestructActionsState) {
  const onSelfDestruct = async () => {
    if (selfDestructConfirm !== 'CONFIRM_SELF_DESTRUCT') {
      showNotice.error('请输入正确的确认码')
      return
    }

    try {
      await securitySelfDestruct(selfDestructConfirm)
    } catch (error) {
      showNotice.error(`自毁失败: ${error}`)
    }
  }

  return { onSelfDestruct }
}
