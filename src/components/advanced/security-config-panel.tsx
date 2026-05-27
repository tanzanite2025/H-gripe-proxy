/**
 * 安全防御配置面板
 */

import { useState } from 'react'
import {
  Box,
  Card,
  CardContent,
  Typography,
  Switch,
  FormControlLabel,
  TextField,
  Stack,
  Divider,
  Alert,
  Chip,
} from '@mui/material'
import { SecurityConfig } from '@/services/coordinator'
import { tlsFingerprintGetAll } from '@/services/tls-fingerprint'

interface Props {
  config: SecurityConfig
  onChange: (config: SecurityConfig) => void
}

export function SecurityConfigPanel({ config, onChange }: Props) {
  const [fingerprints, setFingerprints] = useState<string[]>([])

  // 加载 TLS 指纹列表
  useState(() => {
    tlsFingerprintGetAll().then((fps) => {
      setFingerprints(fps.map((f) => f.name))
    })
  })

  return (
    <Box>
      <Alert severity="info" sx={{ mb: 2 }}>
        安全防御功能可以保护您的代理免受主动探测和恶意扫描。
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
                  启用安全监控
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  启用反调试检测和内存蜜罐
                </Typography>
              </Box>
            }
          />
        </CardContent>
      </Card>

      {/* 反主动探测 */}
      <Card sx={{ mb: 2 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            反主动探测
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            防止 GFW 等审查系统主动探测您的代理服务器
          </Typography>

          <Stack spacing={2}>
            <FormControlLabel
              control={
                <Switch
                  checked={config.anti_probe.enabled}
                  onChange={(e) =>
                    onChange({
                      ...config,
                      anti_probe: {
                        ...config.anti_probe,
                        enabled: e.target.checked,
                      },
                    })
                  }
                />
              }
              label="启用反探测"
            />

            {config.anti_probe.enabled && (
              <>
                <TextField
                  label="时间窗口（秒）"
                  type="number"
                  value={config.anti_probe.time_window}
                  onChange={(e) =>
                    onChange({
                      ...config,
                      anti_probe: {
                        ...config.anti_probe,
                        time_window: parseInt(e.target.value) || 300,
                      },
                    })
                  }
                  helperText="握手暗号的有效时间"
                  fullWidth
                />

                <FormControlLabel
                  control={
                    <Switch
                      checked={config.anti_probe.strict_mode}
                      onChange={(e) =>
                        onChange({
                          ...config,
                          anti_probe: {
                            ...config.anti_probe,
                            strict_mode: e.target.checked,
                          },
                        })
                      }
                    />
                  }
                  label={
                    <Box>
                      <Typography variant="body2">严格模式</Typography>
                      <Typography variant="caption" color="text.secondary">
                        非白名单 IP 直接拒绝连接
                      </Typography>
                    </Box>
                  }
                />

                <Box>
                  <Typography variant="body2" gutterBottom>
                    白名单 IP
                  </Typography>
                  <Typography variant="caption" color="text.secondary">
                    这些 IP 可以直接连接，无需验证
                  </Typography>
                  {/* TODO: 添加 IP 列表编辑器 */}
                </Box>
              </>
            )}
          </Stack>
        </CardContent>
      </Card>

      {/* TLS 指纹伪装 */}
      <Card sx={{ mb: 2 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            TLS 指纹伪装
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            伪装成常见浏览器或应用的 TLS 指纹
          </Typography>

          <Stack spacing={2}>
            <Box>
              <Typography variant="body2" gutterBottom>
                选择指纹
              </Typography>
              <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap' }} useFlexGap>
                <Chip
                  label="不使用"
                  color={!config.tls_fingerprint ? 'primary' : 'default'}
                  onClick={() =>
                    onChange({ ...config, tls_fingerprint: null })
                  }
                />
                {fingerprints.map((name) => (
                  <Chip
                    key={name}
                    label={name}
                    color={
                      config.tls_fingerprint === name ? 'primary' : 'default'
                    }
                    onClick={() =>
                      onChange({ ...config, tls_fingerprint: name })
                    }
                  />
                ))}
              </Stack>
            </Box>

            {config.tls_fingerprint && (
              <Alert severity="success">
                当前使用：{config.tls_fingerprint}
              </Alert>
            )}
          </Stack>
        </CardContent>
      </Card>

      {/* 配置欺骗 */}
      <Card>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            配置欺骗
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            创建假配置文件误导扫描工具
          </Typography>

          <Stack spacing={2}>
            <FormControlLabel
              control={
                <Switch
                  checked={config.config_decoy.enabled}
                  onChange={(e) =>
                    onChange({
                      ...config,
                      config_decoy: {
                        ...config.config_decoy,
                        enabled: e.target.checked,
                      },
                    })
                  }
                />
              }
              label="启用配置欺骗"
            />

            {config.config_decoy.enabled && (
              <Alert severity="warning">
                真实配置将被加密存储，假配置将放置在明显位置
              </Alert>
            )}
          </Stack>
        </CardContent>
      </Card>
    </Box>
  )
}
