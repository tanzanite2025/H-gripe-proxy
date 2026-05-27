/**
 * XDP 代理配置面板（仅 Linux）
 */

import {
  Box,
  Card,
  CardContent,
  Typography,
  Switch,
  FormControlLabel,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Alert,
  Stack,
  TextField,
} from '@mui/material'
import { XdpConfig, XdpMode } from '@/services/coordinator'

interface Props {
  config: XdpConfig
  onChange: (config: XdpConfig) => void
}

const modeLabels: Record<XdpMode, string> = {
  Native: 'Native（原生模式，最快）',
  Skb: 'SKB（兼容模式）',
  Generic: 'Generic（通用模式）',
}

export function XdpConfigPanel({ config, onChange }: Props) {
  return (
    <Box>
      <Alert severity="info" sx={{ mb: 2 }}>
        XDP（eXpress Data Path）是 Linux 内核的高性能数据包处理框架。
        启用后可以获得 10 倍以上的性能提升。
      </Alert>

      <Alert severity="warning" sx={{ mb: 2 }}>
        ⚠️ XDP 需要 root 权限和支持 XDP 的网卡驱动。请确保您的系统满足要求。
      </Alert>

      {/* 总开关 */}
      <Card sx={{ mb: 2 }}>
        <CardContent>
          <FormControlLabel
            control={
              <Switch
                checked={config.enabled}
                onChange={(e) =>
                  onChange({ ...config, enabled: e.target.checked })
                }
              />
            }
            label={
              <Box>
                <Typography variant="body1" sx={{ fontWeight: 'bold' }}>
                  启用 XDP 代理
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  零内核态切换，极致性能
                </Typography>
              </Box>
            }
          />
        </CardContent>
      </Card>

      {config.enabled && (
        <>
          {/* 网卡接口 */}
          <Card sx={{ mb: 2 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                网卡配置
              </Typography>

              <TextField
                label="网卡接口"
                value={config.interface}
                onChange={(e) =>
                  onChange({ ...config, interface: e.target.value })
                }
                helperText="例如：eth0, ens33, wlan0"
                fullWidth
              />
            </CardContent>
          </Card>

          {/* XDP 模式 */}
          <Card sx={{ mb: 2 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                XDP 模式
              </Typography>

              <FormControl fullWidth>
                <InputLabel>模式</InputLabel>
                <Select
                  value={config.mode}
                  label="模式"
                  onChange={(e) =>
                    onChange({
                      ...config,
                      mode: e.target.value as XdpMode,
                    })
                  }
                >
                  {Object.entries(modeLabels).map(([value, label]) => (
                    <MenuItem key={value} value={value}>
                      {label}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <Stack spacing={1} sx={{ mt: 2 }}>
                <Alert severity="success">
                  <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                    Native 模式
                  </Typography>
                  <Typography variant="caption">
                    最快，但需要网卡驱动支持。延迟 ~10μs，吞吐量 50+ Gbps
                  </Typography>
                </Alert>

                <Alert severity="info">
                  <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                    SKB 模式
                  </Typography>
                  <Typography variant="caption">
                    兼容性好，性能略低于 Native
                  </Typography>
                </Alert>

                <Alert severity="warning">
                  <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                    Generic 模式
                  </Typography>
                  <Typography variant="caption">
                    所有网卡都支持，但性能最低
                  </Typography>
                </Alert>
              </Stack>
            </CardContent>
          </Card>

          {/* 队列大小 */}
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                高级设置
              </Typography>

              <TextField
                label="队列大小"
                type="number"
                value={config.queue_size}
                onChange={(e) =>
                  onChange({
                    ...config,
                    queue_size: parseInt(e.target.value) || 4096,
                  })
                }
                helperText="数据包队列大小，默认 4096"
                fullWidth
              />
            </CardContent>
          </Card>
        </>
      )}
    </Box>
  )
}
