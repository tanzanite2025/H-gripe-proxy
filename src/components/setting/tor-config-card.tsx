/**
 * Tor 代理配置卡片
 */

import { CheckCircle as CheckIcon, ChevronDown as ExpandMoreIcon, Copy as CopyIcon, AlertCircle as ErrorIcon, Shield as TorIcon } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { Collapse } from '@/components/tailwind/Collapse'
import { IconButton } from '@/components/tailwind/IconButton'
import { List, ListItem, ListItemText } from '@/components/tailwind/List'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import { torProxyService } from '@/services/tor-proxy'
import { cn } from '@/utils/cn'

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
    <div>
      <div className="mb-2 flex items-center">
        <TorIcon className="mr-1 h-5 w-5" />
        <h6 className="flex-grow text-lg font-bold">
          Tor 代理
        </h6>
        <Switch checked={enabled} onChange={handleEnableChange} />
      </div>

      <Alert severity="warning" className="mb-2">
        Tor 会显著降低网络速度（通常 &lt; 1 Mbps），仅在需要最强隐私保护时使用
      </Alert>

      {enabled && (
        <>
          <div className="mb-3">
            <div className="mb-1.5 text-sm text-gray-500 dark:text-gray-400">
              SOCKS5 代理配置
            </div>

            <div className="space-y-2">
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

              <div className="flex items-center gap-1">
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
                  <CopyIcon className="h-4 w-4" />
                </IconButton>
              </div>
            </div>
          </div>

          <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

          <div className="mb-3">
            <div className="mb-1.5 text-sm text-gray-500 dark:text-gray-400">
              连接状态
            </div>

            <div className="space-y-1.5">
              <div className="flex items-center gap-1">
                <div className="text-sm">Tor 状态:</div>
                {status.enabled ? (
                  <Chip
                    icon={<CheckIcon className="h-3 w-3" />}
                    label="已启用"
                    color="success"
                    size="small"
                  />
                ) : (
                  <Chip icon={<ErrorIcon className="h-3 w-3" />} label="未启用" size="small" />
                )}
              </div>

              <div className="flex items-center gap-1">
                <div className="text-sm">连接状态:</div>
                {status.connected ? (
                  <Chip
                    icon={<CheckIcon className="h-3 w-3" />}
                    label="已连接"
                    color="success"
                    size="small"
                  />
                ) : (
                  <Chip icon={<ErrorIcon className="h-3 w-3" />} label="未连接" color="error" size="small" />
                )}
              </div>

              <Button
                variant="outlined"
                size="small"
                onClick={handleCheckConnection}
                fullWidth
              >
                检查连接
              </Button>
            </div>
          </div>
        </>
      )}

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div>
        <Button
          onClick={() => setShowInstructions(!showInstructions)}
          endIcon={
            <ExpandMoreIcon
              className={cn(
                'h-4 w-4 transition-transform duration-300',
                showInstructions ? 'rotate-180' : 'rotate-0',
              )}
            />
          }
          fullWidth
        >
          使用说明
        </Button>

        <Collapse in={showInstructions}>
          <div className="mt-2">
            <div className="mb-1 text-sm font-medium">
              {instructions.title}
            </div>

            <List dense>
              {instructions.steps.map((step, index) => (
                <ListItem key={index}>
                  <ListItemText primary={step} />
                </ListItem>
              ))}
            </List>

            <div className="mb-1 mt-2 text-sm font-medium">
              注意事项
            </div>

            <List dense>
              {instructions.notes.map((note, index) => (
                <ListItem key={index}>
                  <ListItemText primary={note} />
                </ListItem>
              ))}
            </List>
          </div>
        </Collapse>
      </div>
    </div>
  )
}
