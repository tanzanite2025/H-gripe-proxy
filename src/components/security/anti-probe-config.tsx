/**
 * 反主动探测配置组件
 */

import {
  Box,
  Button,
  Chip,
  FormControlLabel,
  Paper,
  Stack,
  Switch,
  TextField,
  Typography,
} from '@mui/material'
import {
  ContentCopyOutlined,
  DeleteOutlined,
  RefreshOutlined,
  SecurityOutlined,
  WarningAmberOutlined,
} from '@mui/icons-material'
import { useEffect, useState } from 'react'

import {
  type AntiProbeConfig,
  antiProbeCleanup,
  antiProbeGenerateToken,
  antiProbeGetConfig,
  antiProbeUpdateConfig,
} from '@/services/anti-probe'
import { showNotice } from '@/services/notice-service'

export default function AntiProbeConfigComponent() {
  const [config, setConfig] = useState<AntiProbeConfig>({
    enabled: false,
    secret_key: '',
    time_window: 300,
    whitelist: [],
    strict_mode: false,
  })
  const [newIp, setNewIp] = useState('')
  const [token, setToken] = useState('')
  const [loading, setLoading] = useState(false)

  // 加载配置
  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    try {
      const cfg = await antiProbeGetConfig()
      setConfig(cfg)
    } catch (error) {
      showNotice.error(`加载配置失败: ${error}`)
    }
  }

  // 保存配置
  const handleSave = async () => {
    try {
      setLoading(true)
      await antiProbeUpdateConfig(config)
      showNotice.success('配置已保存')
    } catch (error) {
      showNotice.error(`保存配置失败: ${error}`)
    } finally {
      setLoading(false)
    }
  }

  // 生成新密钥
  const handleGenerateKey = () => {
    const newKey = Array.from({ length: 32 }, () =>
      Math.floor(Math.random() * 256)
        .toString(16)
        .padStart(2, '0'),
    ).join('')
    setConfig({ ...config, secret_key: newKey })
  }

  // 生成握手暗号
  const handleGenerateToken = async () => {
    try {
      const newToken = await antiProbeGenerateToken()
      setToken(newToken)
      showNotice.success('握手暗号已生成')
    } catch (error) {
      showNotice.error(`生成暗号失败: ${error}`)
    }
  }

  // 复制暗号
  const handleCopyToken = () => {
    navigator.clipboard.writeText(token)
    showNotice.success('已复制到剪贴板')
  }

  // 添加白名单 IP
  const handleAddIp = () => {
    if (!newIp.trim()) return

    // 简单的 IP 格式验证
    const ipRegex =
      /^(\d{1,3}\.){3}\d{1,3}$|^([0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}$/
    if (!ipRegex.test(newIp)) {
      showNotice.error('无效的 IP 地址格式')
      return
    }

    if (config.whitelist.includes(newIp)) {
      showNotice.error('IP 已存在于白名单')
      return
    }

    setConfig({
      ...config,
      whitelist: [...config.whitelist, newIp],
    })
    setNewIp('')
  }

  // 删除白名单 IP
  const handleRemoveIp = (ip: string) => {
    setConfig({
      ...config,
      whitelist: config.whitelist.filter((i) => i !== ip),
    })
  }

  // 清理过期缓存
  const handleCleanup = async () => {
    try {
      await antiProbeCleanup()
      showNotice.success('已清理过期缓存')
    } catch (error) {
      showNotice.error(`清理失败: ${error}`)
    }
  }

  return (
    <Box sx={{ p: 3 }}>
      <Stack spacing={3}>
        {/* 标题 */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <SecurityOutlined color="primary" />
          <Typography variant="h6">反主动探测配置</Typography>
        </Box>

        {/* 说明 */}
        <Paper sx={{ p: 2, bgcolor: 'warning.main', color: 'warning.contrastText' }}>
          <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
            <WarningAmberOutlined />
            <Box>
              <Typography variant="body2" sx={{ fontWeight: 600 }}>
                幻影无响应模式
              </Typography>
              <Typography variant="caption">
                对未携带握手暗号的连接直接丢弃，不返回任何响应。在外部探测者看来，服务器就像一个完全不存在的"黑洞
                IP"。
              </Typography>
            </Box>
          </Box>
        </Paper>

        {/* 基础配置 */}
        <Paper sx={{ p: 2 }}>
          <Stack spacing={2}>
            <FormControlLabel
              control={
                <Switch
                  checked={config.enabled}
                  onChange={(e) =>
                    setConfig({ ...config, enabled: e.target.checked })
                  }
                />
              }
              label="启用反主动探测"
            />

            <FormControlLabel
              control={
                <Switch
                  checked={config.strict_mode}
                  onChange={(e) =>
                    setConfig({ ...config, strict_mode: e.target.checked })
                  }
                  disabled={!config.enabled}
                />
              }
              label="严格模式（非白名单直接拒绝）"
            />

            <TextField
              label="时间窗口（秒）"
              type="number"
              value={config.time_window}
              onChange={(e) =>
                setConfig({
                  ...config,
                  time_window: Number.parseInt(e.target.value),
                })
              }
              disabled={!config.enabled}
              helperText="握手暗号的有效时间"
              fullWidth
            />
          </Stack>
        </Paper>

        {/* 密钥管理 */}
        <Paper sx={{ p: 2 }}>
          <Stack spacing={2}>
            <Typography variant="subtitle2">私钥管理</Typography>
            <TextField
              label="私钥"
              value={config.secret_key}
              onChange={(e) =>
                setConfig({ ...config, secret_key: e.target.value })
              }
              disabled={!config.enabled}
              fullWidth
              slotProps={{
                input: {
                  readOnly: true,
                  sx: { fontFamily: 'monospace', fontSize: '0.875rem' },
                },
              }}
            />
            <Button
              variant="outlined"
              startIcon={<RefreshOutlined />}
              onClick={handleGenerateKey}
              disabled={!config.enabled}
            >
              生成新密钥
            </Button>
          </Stack>
        </Paper>

        {/* 握手暗号生成 */}
        <Paper sx={{ p: 2 }}>
          <Stack spacing={2}>
            <Typography variant="subtitle2">握手暗号生成</Typography>
            <Button
              variant="contained"
              startIcon={<RefreshOutlined />}
              onClick={handleGenerateToken}
              disabled={!config.enabled}
            >
              生成握手暗号
            </Button>
            {token && (
              <Box>
                <TextField
                  label="当前暗号"
                  value={token}
                  fullWidth
                  slotProps={{
                    input: {
                      readOnly: true,
                      sx: { fontFamily: 'monospace', fontSize: '0.875rem' },
                      endAdornment: (
                        <Button
                          size="small"
                          startIcon={<ContentCopyOutlined />}
                          onClick={handleCopyToken}
                        >
                          复制
                        </Button>
                      ),
                    },
                  }}
                />
                <Typography variant="caption" color="text.secondary">
                  此暗号在 {config.time_window} 秒内有效
                </Typography>
              </Box>
            )}
          </Stack>
        </Paper>

        {/* 白名单管理 */}
        <Paper sx={{ p: 2 }}>
          <Stack spacing={2}>
            <Typography variant="subtitle2">IP 白名单</Typography>
            <Box sx={{ display: 'flex', gap: 1 }}>
              <TextField
                label="添加 IP 地址"
                value={newIp}
                onChange={(e) => setNewIp(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleAddIp()}
                disabled={!config.enabled}
                placeholder="192.168.1.1 或 2001:db8::1"
                fullWidth
              />
              <Button
                variant="contained"
                onClick={handleAddIp}
                disabled={!config.enabled}
              >
                添加
              </Button>
            </Box>
            <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
              {config.whitelist.map((ip) => (
                <Chip
                  key={ip}
                  label={ip}
                  onDelete={() => handleRemoveIp(ip)}
                  deleteIcon={<DeleteOutlined />}
                  disabled={!config.enabled}
                />
              ))}
              {config.whitelist.length === 0 && (
                <Typography variant="body2" color="text.secondary">
                  暂无白名单 IP
                </Typography>
              )}
            </Box>
          </Stack>
        </Paper>

        {/* 操作按钮 */}
        <Box sx={{ display: 'flex', gap: 2 }}>
          <Button
            variant="contained"
            onClick={handleSave}
            disabled={loading}
            fullWidth
          >
            保存配置
          </Button>
          <Button
            variant="outlined"
            startIcon={<DeleteOutlined />}
            onClick={handleCleanup}
            disabled={!config.enabled}
          >
            清理缓存
          </Button>
        </Box>
      </Stack>
    </Box>
  )
}
