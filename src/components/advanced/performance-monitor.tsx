/**
 * 性能监控面板
 */

import {
  Box,
  Card,
  CardContent,
  Typography,
  Alert,
  Button,
  Stack,
  Chip,
} from '@mui/material'
import {
  CheckCircleOutlined,
  ErrorOutlined,
  WarningOutlined,
  RefreshOutlined,
} from '@mui/icons-material'
import { CoordinatorStatus } from '@/services/coordinator'

interface Props {
  status: CoordinatorStatus | null
  onRefresh: () => void
}

export function PerformanceMonitor({ status, onRefresh }: Props) {
  if (!status) {
    return (
      <Box>
        <Alert severity="info">加载中...</Alert>
      </Box>
    )
  }

  return (
    <Box>
      {/* 安全状态警告 */}
      {status.security_compromised && (
        <Alert severity="error" sx={{ mb: 2 }}>
          <Typography variant="body1" sx={{ fontWeight: 'bold' }}>
            ⚠️ 安全状态已被破坏
          </Typography>
          <Typography variant="body2">
            检测到调试器或恶意扫描。建议立即停止使用并检查系统安全。
          </Typography>
        </Alert>
      )}

      {/* 刷新按钮 */}
      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mb: 2 }}>
        <Button
          variant="outlined"
          startIcon={<RefreshOutlined />}
          onClick={onRefresh}
        >
          刷新状态
        </Button>
      </Box>

      {/* 模块状态 */}
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: { xs: '1fr', md: 'repeat(2, 1fr)' },
          gap: 2,
        }}
      >
        {/* 协调器状态 */}
        <Card>
          <CardContent>
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mb: 2 }}>
              {status.initialized ? (
                <CheckCircleOutlined color="success" />
              ) : (
                <ErrorOutlined color="error" />
              )}
              <Typography variant="h6">核心协调器</Typography>
            </Stack>

            <Stack spacing={1}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <Typography variant="body2">状态</Typography>
                <Chip
                  label={status.initialized ? '已初始化' : '未初始化'}
                  size="small"
                  color={status.initialized ? 'success' : 'error'}
                />
              </Box>
            </Stack>
          </CardContent>
        </Card>

        {/* 安全监控 */}
        <Card>
          <CardContent>
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mb: 2 }}>
              {status.security_enabled && !status.security_compromised ? (
                <CheckCircleOutlined color="success" />
              ) : status.security_compromised ? (
                <ErrorOutlined color="error" />
              ) : (
                <WarningOutlined color="warning" />
              )}
              <Typography variant="h6">安全监控</Typography>
            </Stack>

            <Stack spacing={1}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <Typography variant="body2">状态</Typography>
                <Chip
                  label={
                    status.security_enabled
                      ? status.security_compromised
                        ? '已破坏'
                        : '运行中'
                      : '未启用'
                  }
                  size="small"
                  color={
                    status.security_enabled
                      ? status.security_compromised
                        ? 'error'
                        : 'success'
                      : 'default'
                  }
                />
              </Box>
            </Stack>
          </CardContent>
        </Card>

        {/* 反探测 */}
        <Card>
          <CardContent>
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mb: 2 }}>
              {status.anti_probe_enabled ? (
                <CheckCircleOutlined color="success" />
              ) : (
                <WarningOutlined color="warning" />
              )}
              <Typography variant="h6">反主动探测</Typography>
            </Stack>

            <Stack spacing={1}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <Typography variant="body2">状态</Typography>
                <Chip
                  label={status.anti_probe_enabled ? '已启用' : '未启用'}
                  size="small"
                  color={status.anti_probe_enabled ? 'success' : 'default'}
                />
              </Box>
            </Stack>
          </CardContent>
        </Card>

        {/* TLS 指纹 */}
        <Card>
          <CardContent>
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mb: 2 }}>
              {status.tls_fingerprint ? (
                <CheckCircleOutlined color="success" />
              ) : (
                <WarningOutlined color="warning" />
              )}
              <Typography variant="h6">TLS 指纹伪装</Typography>
            </Stack>

            <Stack spacing={1}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <Typography variant="body2">当前指纹</Typography>
                <Chip
                  label={status.tls_fingerprint || '未设置'}
                  size="small"
                  color={status.tls_fingerprint ? 'success' : 'default'}
                />
              </Box>
            </Stack>
          </CardContent>
        </Card>

        {/* 多路径路由 */}
        <Card>
          <CardContent>
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mb: 2 }}>
              {status.multipath_enabled ? (
                <CheckCircleOutlined color="success" />
              ) : (
                <WarningOutlined color="warning" />
              )}
              <Typography variant="h6">多路径路由</Typography>
            </Stack>

            <Stack spacing={1}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <Typography variant="body2">状态</Typography>
                <Chip
                  label={status.multipath_enabled ? '已启用' : '未启用'}
                  size="small"
                  color={status.multipath_enabled ? 'success' : 'default'}
                />
              </Box>
            </Stack>
          </CardContent>
        </Card>

        {/* XDP 代理（仅 Linux） */}
        {status.xdp_enabled !== undefined && (
          <Card>
            <CardContent>
              <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mb: 2 }}>
                {status.xdp_enabled && status.xdp_running ? (
                  <CheckCircleOutlined color="success" />
                ) : status.xdp_enabled ? (
                  <WarningOutlined color="warning" />
                ) : (
                  <WarningOutlined color="warning" />
                )}
                <Typography variant="h6">XDP 代理</Typography>
              </Stack>

              <Stack spacing={1}>
                <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                  <Typography variant="body2">状态</Typography>
                  <Chip
                    label={
                      status.xdp_enabled
                        ? status.xdp_running
                          ? '运行中'
                          : '已启用但未运行'
                        : '未启用'
                    }
                    size="small"
                    color={
                      status.xdp_enabled && status.xdp_running
                        ? 'success'
                        : status.xdp_enabled
                        ? 'warning'
                        : 'default'
                    }
                  />
                </Box>
              </Stack>
            </CardContent>
          </Card>
        )}
      </Box>

      {/* 性能提示 */}
      <Card sx={{ mt: 2 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            性能优化建议
          </Typography>

          <Stack spacing={1}>
            {!status.security_enabled && (
              <Alert severity="info">
                建议启用安全监控以保护您的代理
              </Alert>
            )}

            {!status.anti_probe_enabled && (
              <Alert severity="info">
                建议启用反探测以防止主动探测
              </Alert>
            )}

            {!status.tls_fingerprint && (
              <Alert severity="info">
                建议设置 TLS 指纹伪装以提高隐蔽性
              </Alert>
            )}

            {status.xdp_enabled !== undefined && !status.xdp_enabled && (
              <Alert severity="info">
                Linux 系统可以启用 XDP 代理获得 10 倍性能提升
              </Alert>
            )}

            {status.security_enabled &&
              status.anti_probe_enabled &&
              status.tls_fingerprint &&
              status.multipath_enabled && (
                <Alert severity="success">
                  ✅ 所有高级功能已启用，您的代理处于最佳状态
                </Alert>
              )}
          </Stack>
        </CardContent>
      </Card>
    </Box>
  )
}
