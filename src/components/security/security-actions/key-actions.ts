import { showNotice } from '@/services/notice-service'
import { securityGenerateEncryptionKey } from '@/services/security'

interface KeyActionsState {
  encryptionKey: string
  setEncryptionKey: (key: string) => void
}

export function createKeyActions({
  encryptionKey,
  setEncryptionKey,
}: KeyActionsState) {
  const onGenerateKey = async () => {
    try {
      const key = await securityGenerateEncryptionKey()
      setEncryptionKey(key)
      showNotice.success('加密密钥已生成')
    } catch (error) {
      showNotice.error(`生成失败: ${error}`)
    }
  }

  const onCopyKey = () => {
    void navigator.clipboard.writeText(encryptionKey)
    showNotice.success('已复制到剪贴板')
  }

  return {
    onGenerateKey,
    onCopyKey,
  }
}
