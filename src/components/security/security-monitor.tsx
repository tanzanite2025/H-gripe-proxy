/**
 * 安全监控组件
 */

import {
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  FormControlLabel,
  Paper,
  Stack,
  Switch,
  TextField,
  Typography,
} from '@mui/material'
import {
  BugReportOutlined,
  ContentCopyOutlined,
  DeleteForeverOutlined,
  SecurityOutlined,
  WarningAmberOutlined,
} from '@mui/icons-material'
import { useEffect, useState } from 'react'

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
import { showNotice } from '@/services/notice-service'

export default function SecurityMonitor() {
  const [monitorEnabled, setMonitorEnabled] = useState(false)
  const [status, setStatus] = useState<SecurityStatus>({
    compromised: false,
    debugger_present: false,
    memory_scanning: false,
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
    <Box sx={{ p: 3 }}>
      <Stack spacing={3}>
        {/* 标题 */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <SecurityOutlined color="primary" />
          <Typography variant="h6">内生欺骗陷阱（Canary Honeytoken）</Typography>
        </Box>

        {/* 说明 */}
        <Paper
          sx={{ p: 2, bgcolor: 'error.main', color: 'error.contrastText' }}
        >
          <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
            <WarningAmberOutlined />
            <Box>
              <Typography variant="body2" sx={{ fontWeight: 600 }}>
                防御究极体
              </Typography>
              <Typography variant="caption">
                反调试、内存蜜罐、配置欺骗、自毁机制 -
                全方位防范本地流氓软件扫描和物理攻破
              </Typography>
            </Box>
          </Box>
        </Paper>

        {/* 安全状态 */}
        <Card>
          <CardContent>
            <Typography variant="subtitle2" sx={{ mb: 2 }}>
              安全状态监控
            </Typography>
            <Stack spacing={2}>
              <FormControlLabel
                control={
                  <Switch
                    checked={monitorEnabled}
                    onChange={handleToggleMonitor}
                  />
                }
                label="启用安全监控"
              />

              <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                <Chip
                  label={status.compromised ? '已破坏' : '安全'}
                  color={status.compromised ? 'error' : 'success'}
                  icon={<SecurityOutlined />}
                />
                <Chip
                  label={status.debugger_present ? '检测到调试器' : '无调试器'}
                  color={status.debugger_present ? 'error' : 'default'}
                  icon={<BugReportOutlined />}
                />
                <Chip
                  label={
                    status.memory_scanning ? '检测到内存扫描' : '无内存扫描'
                  }
                  color={status.memory_scanning ? 'error' : 'default'}
                />
              </Box>
            </Stack>
          </CardContent>
        </Card>

        {/* 配置文件欺骗 */}
        <Card>
          <CardContent>
            <Typography variant="subtitle2" sx={{ mb: 2 }}>
              配置文件欺骗
            </Typography>
            <Stack spacing={2}>
              <TextField
                label="假配置文件路径"
                value={decoyPath}
                onChange={(e) => setDecoyPath(e.target.value)}
                fullWidth
                helperText="放置假配置文件来误导扫描软件"
              />
              <Box sx={{ display: 'flex', gap: 1 }}>
                <Button variant="contained" onClick={handleDeployDecoy}>
                  部署假配置
                </Button>
                <Button variant="outlined" onClick={handleCheckDecoyAccess}>
                  检查访问
                </Button>
                <Button
                  variant="outlined"
                  color="error"
                  onClick={handleCleanupDecoy}
                >
                  清除假配置
                </Button>
              </Box>
            </Stack>
          </CardContent>
        </Card>

        {/* 加密密钥管理 */}
        <Card>
          <CardContent>
            <Typography variant="subtitle2" sx={{ mb: 2 }}>
              加密密钥管理
            </Typography>
            <Stack spacing={2}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Chip
                  label={hasEncryptionKey ? '密钥已设置' : '密钥未设置'}
                  color={hasEncryptionKey ? 'success' : 'warning'}
                />
                <Typography variant="caption" color="text.secondary">
                  真实配置只在内存中加密存储
                </Typography>
              </Box>

              <Button variant="contained" onClick={handleGenerateKey}>
                生成新密钥
              </Button>

              {encryptionKey && (
                <Box>
                  <TextField
                    label="加密密钥（请保存到环境变量）"
                    value={encryptionKey}
                    fullWidth
                    slotProps={{
                      input: {
                        readOnly: true,
                        sx: { fontFamily: 'monospace', fontSize: '0.75rem' },
                        endAdornment: (
                          <Button
                            size="small"
                            startIcon={<ContentCopyOutlined />}
                            onClick={handleCopyKey}
                          >
                            复制
                          </Button>
                        ),
                      },
                    }}
                  />
                  <Typography variant="caption" color="warning.main">
                    请将此密钥设置为环境变量 CLASH_VERGE_SECURE_KEY
                  </Typography>
                </Box>
              )}
            </Stack>
          </CardContent>
        </Card>

        {/* 自毁机制 */}
        <Card sx={{ borderColor: 'error.main', borderWidth: 2 }}>
          <CardContent>
            <Typography variant="subtitle2" sx={{ mb: 2, color: 'error.main' }}>
              🚨 紧急自毁机制
            </Typography>
            <Stack spacing={2}>
              <Typography variant="body2" color="text.secondary">
                检测到安全威胁时，自动清除内存中的密钥、擦除本地缓存并退出程序
              </Typography>

              <TextField
                label="确认码"
                value={selfDestructConfirm}
                onChange={(e) => setSelfDestructConfirm(e.target.value)}
                placeholder="输入 CONFIRM_SELF_DESTRUCT"
                fullWidth
                helperText="手动触发自毁需要输入确认码"
              />

              <Button
                variant="contained"
                color="error"
                startIcon={<DeleteForeverOutlined />}
                onClick={handleSelfDestruct}
                disabled={selfDestructConfirm !== 'CONFIRM_SELF_DESTRUCT'}
              >
                触发自毁
              </Button>
            </Stack>
          </CardContent>
        </Card>
      </Stack>
    </Box>
  )
}
