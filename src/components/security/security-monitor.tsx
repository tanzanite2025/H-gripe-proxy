/**
 * 安全监控组件
 */

import { useEffect, useState } from 'react'

import { showNotice } from '@/services/notice-service'
import {
  type SecurityStatus,
  securityCheckDecoyAccess,
  securityCheckEncryptionKey,
  securityCheckStatus,
  securityCleanupDecoy,
  securityDeployDecoy,
  securityGenerateEncryptionKey,
  securitySelfDestruct,
  securityStartMonitor,
  securityStopMonitor,
} from '@/services/security'

import SecurityMonitorUI from './security-monitor-ui'

export default function SecurityMonitor() {
  const [monitorEnabled, setMonitorEnabled] = useState(false)
  const [status, setStatus] = useState<SecurityStatus>({
    compromised: false,
    debugger_present: false,
    memory_scanning: false,
    leak_detected: false,
    leak_type: null,
  })
  const [decoyPath, setDecoyPath] = useState('config_decoy.yaml')
  const [encryptionKey, setEncryptionKey] = useState('')
  const [hasEncryptionKey, setHasEncryptionKey] = useState(false)
  const [selfDestructConfirm, setSelfDestructConfirm] = useState('')

  // 定期检查安全状态
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const newStatus = await securityCheckStatus()
        setStatus(newStatus)

        if (newStatus.compromised) {
          showNotice.error('🚨 安全状态已被破坏！')
        }
        if (newStatus.leak_detected) {
          showNotice.error(`🚨 泄漏检测: ${newStatus.leak_type ?? '未知类型'}`)
        }
      } catch (error) {
        console.error('检查安全状态失败:', error)
      }
    }, 5000)

    return () => clearInterval(interval)
  }, [])

  // 检查加密密钥
  useEffect(() => {
    checkEncryptionKey()
  }, [])

  const checkEncryptionKey = async () => {
    try {
      const hasKey = await securityCheckEncryptionKey()
      setHasEncryptionKey(hasKey)
    } catch (error) {
      console.error('检查加密密钥失败:', error)
    }
  }

  // 启动/停止监控
  const handleToggleMonitor = async () => {
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

  // 部署假配置
  const handleDeployDecoy = async () => {
    try {
      await securityDeployDecoy(decoyPath)
      showNotice.success('假配置文件已部署')
    } catch (error) {
      showNotice.error(`部署失败: ${error}`)
    }
  }

  // 清除假配置
  const handleCleanupDecoy = async () => {
    try {
      await securityCleanupDecoy(decoyPath)
      showNotice.success('假配置文件已清除')
    } catch (error) {
      showNotice.error(`清除失败: ${error}`)
    }
  }

  // 检查假配置访问
  const handleCheckDecoyAccess = async () => {
    try {
      const accessed = await securityCheckDecoyAccess(decoyPath)
      if (accessed) {
        showNotice.error('🚨 假配置文件被访问！')
      } else {
        showNotice.success('假配置文件未被访问')
      }
    } catch (error) {
      showNotice.error(`检查失败: ${error}`)
    }
  }

  // 生成加密密钥
  const handleGenerateKey = async () => {
    try {
      const key = await securityGenerateEncryptionKey()
      setEncryptionKey(key)
      showNotice.success('加密密钥已生成')
    } catch (error) {
      showNotice.error(`生成失败: ${error}`)
    }
  }

  // 复制密钥
  const handleCopyKey = () => {
    navigator.clipboard.writeText(encryptionKey)
    showNotice.success('已复制到剪贴板')
  }

  // 触发自毁
  const handleSelfDestruct = async () => {
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

  return (
    <SecurityMonitorUI
      monitorEnabled={monitorEnabled}
      status={status}
      decoyPath={decoyPath}
      encryptionKey={encryptionKey}
      hasEncryptionKey={hasEncryptionKey}
      selfDestructConfirm={selfDestructConfirm}
      onToggleMonitor={handleToggleMonitor}
      onDecoyPathChange={setDecoyPath}
      onDeployDecoy={handleDeployDecoy}
      onCleanupDecoy={handleCleanupDecoy}
      onCheckDecoyAccess={handleCheckDecoyAccess}
      onGenerateKey={handleGenerateKey}
      onCopyKey={handleCopyKey}
      onSelfDestructConfirmChange={setSelfDestructConfirm}
      onSelfDestruct={handleSelfDestruct}
    />
  )
}
