/**
 * Tor 代理配置卡片
 */

import { useState, useEffect } from 'react'
import {
  Box,
  Typography,
  Switch,
  TextField,
  Button,
  Stack,
  Divider,
  Alert,
  Chip,
  List,
  ListItem,
  ListItemText,
  IconButton,
  Collapse,
} from '@mui/material'
import {
  VpnLock as TorIcon,
  CheckCircle as CheckIcon,
  Error as ErrorIcon,
  ExpandMore as ExpandMoreIcon,
  ContentCopy as CopyIcon,
} from '@mui/icons-material'
import { torProxyService } from '@/services/tor-proxy'

export const TorConfigCard = () => {
  const [enabled, setEnabled] = useState(false)
  const [socksHost, setSocksHost] = useState('127.0.0.1')
  const [socksPort, setSocksPort] = useState(9050)
  const [status, setStatus] = useState({
    enabled: false,
    connected: false,
    circuitEstablished: false,
  })
  const [showInstructions, setShowInstructions] = useState(false)

  useEffect(() => {
    // 初始化
    const config = torProxyService.getConfig()
    setEnabled(config.enabled)
    setSocksHost(config.socksHost)
    setSocksPort(config.socksPort)
    updateStatus()

    // 定期更新状态
    const interval = setInterval(updateStatus, 5000)
    return () => clearInterval(interval)
  }, [])

  const updateStatus = () => {
    const newStatus = torProxyService.getStatus()
    setStatus(newStatus)
  }

  const handleEnableChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const newEnabled = event.target.checked
    setEnabled(newEnabled)

    if (newEnabled) {
      torProxyService.enable({
        socksHost,
        socksPort,
      })
    } else {
      torProxyService.disable()
    }

    updateStatus()
  }

  const handleSocksHostChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSocksHost(event.target.value)
    if (enabled) {
      torProxyService.setConfig({ socksHost: event.target.value })
    }
  }

  const handleSocksPortChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const port = parseInt(event.target.value)
    if (!isNaN(port)) {
      setSocksPort(port)
      if (enabled) {
        torProxyService.setConfig({ socksPort: port })
      }
    }
  }

  const handleCheckConnection = async () => {
    await torProxyService.checkConnection()
    updateStatus()
  }

  const handleCopySocksUrl = () => {
    const url = torProxyService.getSocksProxyUrl()
    navigator.clipboard.writeText(url)
  }

  const instructions = torProxyService.getUsageInstructions()

  return (
    <Box>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
        <TorIcon sx={{ mr: 1 }} />
        <Typography variant="h6" sx={{ fontWeight: 'bold', flexGrow: 1 }}>
          Tor 代理
        </Typography>
        <Switch checked={enabled} onChange={handleEnableChange} />
      </Box>

      <Alert severity="warning" sx={{ mb: 2 }}>
        Tor 会显著降低网络速度（通常 &lt; 1 Mbps），仅在需要最强隐私保护时使用
      </Alert>

      {enabled && (
        <>
          <Box sx={{ mb: 3 }}>
            <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
              SOCKS5 代理配置
            </Typography>

            <Stack spacing={2}>
              <TextField
                label="SOCKS5 主机"
                value={socksHost}
                onChange={handleSocksHostChange}
                size="small"
                fullWidth
              />

              <TextField
                label="SOCKS5 端口"
                type="number"
                value={socksPort}
                onChange={handleSocksPortChange}
                size="small"
                fullWidth
              />

              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <TextField
                  label="SOCKS5 代理地址"
                  value={torProxyService.getSocksProxyUrl()}
                  size="small"
                  fullWidth
                  slotProps={{
                    input: {
                      readOnly: true,
                    },
                  }}
                />
                <IconButton onClick={handleCopySocksUrl} size="small">
                  <CopyIcon />
                </IconButton>
              </Box>
            </Stack>
          </Box>

          <Divider sx={{ my: 2 }} />

          <Box sx={{ mb: 3 }}>
            <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
              连接状态
            </Typography>

            <Stack spacing={1.5}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Typography variant="body2">Tor 状态:</Typography>
                {status.enabled ? (
                  <Chip
                    icon={<CheckIcon />}
                    label="已启用"
                    color="success"
                    size="small"
                  />
                ) : (
                  <Chip icon={<ErrorIcon />} label="未启用" size="small" />
                )}
              </Box>

              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Typography variant="body2">连接状态:</Typography>
                {status.connected ? (
                  <Chip
                    icon={<CheckIcon />}
                    label="已连接"
                    color="success"
                    size="small"
                  />
                ) : (
                  <Chip icon={<ErrorIcon />} label="未连接" color="error" size="small" />
                )}
              </Box>

              <Button
                variant="outlined"
                size="small"
                onClick={handleCheckConnection}
                fullWidth
              >
                检查连接
              </Button>
            </Stack>
          </Box>
        </>
      )}

      <Divider sx={{ my: 2 }} />

      <Box>
        <Button
          onClick={() => setShowInstructions(!showInstructions)}
          endIcon={
            <ExpandMoreIcon
              sx={{
                transform: showInstructions ? 'rotate(180deg)' : 'rotate(0deg)',
                transition: '0.3s',
              }}
            />
          }
          fullWidth
        >
          使用说明
        </Button>

        <Collapse in={showInstructions}>
          <Box sx={{ mt: 2 }}>
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              {instructions.title}
            </Typography>

            <List dense>
              {instructions.steps.map((step, index) => (
                <ListItem key={index}>
                  <ListItemText primary={step} />
                </ListItem>
              ))}
            </List>

            <Typography variant="subtitle2" sx={{ mt: 2, mb: 1 }}>
              注意事项
            </Typography>

            <List dense>
              {instructions.notes.map((note, index) => (
                <ListItem key={index}>
                  <ListItemText primary={note} />
                </ListItem>
              ))}
            </List>
          </Box>
        </Collapse>
      </Box>
    </Box>
  )
}
