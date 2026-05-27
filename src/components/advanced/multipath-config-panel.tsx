/**
 * 多路径路由配置面板
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
  Button,
  Chip,
} from '@mui/material'
import { MultipathConfig, SlicingStrategy, PoolType } from '@/services/coordinator'

interface Props {
  config: MultipathConfig
  onChange: (config: MultipathConfig) => void
}

const strategyLabels: Record<SlicingStrategy, string> = {
  RoundRobin: '轮询（均匀分配）',
  Random: '随机',
  Weighted: '加权（推荐）',
  LeastConnections: '最少连接',
  LatencyBased: '延迟优先',
}

const poolTypeLabels: Record<PoolType, string> = {
  General: '通用池',
  Streaming: '流媒体专用',
  Gaming: '游戏专用',
  Download: '下载专用',
  Social: '社交媒体',
}

export function MultipathConfigPanel({ config, onChange }: Props) {
  return (
    <Box>
      <Alert severity="warning" sx={{ mb: 2 }}>
        ⚠️ 多路径路由会将流量分片到多个节点。流媒体和游戏服务会自动使用单节点模式，避免 IP 变化导致封号。
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
                <Typography variant="body1" fontWeight="bold">
                  启用多路径路由
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  将流量分片到多个节点，降维打击行为分析
                </Typography>
              </Box>
            }
          />
        </CardContent>
      </Card>

      {config.enabled && (
        <>
          {/* 分片策略 */}
          <Card sx={{ mb: 2 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                分片策略
              </Typography>

              <FormControl fullWidth>
                <InputLabel>策略</InputLabel>
                <Select
                  value={config.strategy}
                  label="策略"
                  onChange={(e) =>
                    onChange({
                      ...config,
                      strategy: e.target.value as SlicingStrategy,
                    })
                  }
                >
                  {Object.entries(strategyLabels).map(([value, label]) => (
                    <MenuItem key={value} value={value}>
                      {label}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
                {config.strategy === 'Weighted' && '推荐：根据节点权重分配流量'}
                {config.strategy === 'RoundRobin' && '轮询：均匀分配到所有节点'}
                {config.strategy === 'Random' && '随机：完全随机选择节点'}
                {config.strategy === 'LeastConnections' && '最少连接：选择连接数最少的节点'}
                {config.strategy === 'LatencyBased' && '延迟优先：选择延迟最低的节点'}
              </Typography>
            </CardContent>
          </Card>

          {/* 会话保持 */}
          <Card sx={{ mb: 2 }}>
            <CardContent>
              <FormControlLabel
                control={
                  <Switch
                    checked={config.session_persistence}
                    onChange={(e) =>
                      onChange({
                        ...config,
                        session_persistence: e.target.checked,
                      })
                    }
                  />
                }
                label={
                  <Box>
                    <Typography variant="body1" fontWeight="bold">
                      会话保持
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      同一会话使用相同节点（推荐开启）
                    </Typography>
                  </Box>
                }
              />
            </CardContent>
          </Card>

          {/* 节点池 */}
          <Card sx={{ mb: 2 }}>
            <CardContent>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
                <Typography variant="h6">
                  节点池
                </Typography>
                <Button variant="outlined" size="small">
                  添加节点池
                </Button>
              </Box>

              {config.node_pools.length === 0 ? (
                <Alert severity="info">
                  还没有节点池。点击"添加节点池"开始配置。
                </Alert>
              ) : (
                <Stack spacing={2}>
                  {config.node_pools.map((pool, index) => (
                    <Card key={index} variant="outlined">
                      <CardContent>
                        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                          <Box>
                            <Typography variant="body1" fontWeight="bold">
                              {pool.name}
                            </Typography>
                            <Stack direction="row" spacing={1} sx={{ mt: 1 }}>
                              <Chip
                                label={poolTypeLabels[pool.pool_type]}
                                size="small"
                                color="primary"
                              />
                              <Chip
                                label={`${pool.nodes.length} 个节点`}
                                size="small"
                              />
                              <Chip
                                label={pool.enabled ? '已启用' : '已禁用'}
                                size="small"
                                color={pool.enabled ? 'success' : 'default'}
                              />
                            </Stack>
                          </Box>
                          <Box>
                            <Button size="small">编辑</Button>
                            <Button size="small" color="error">
                              删除
                            </Button>
                          </Box>
                        </Box>
                      </CardContent>
                    </Card>
                  ))}
                </Stack>
              )}
            </CardContent>
          </Card>

          {/* 预定义规则 */}
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                会话绑定规则
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                以下服务会自动使用单节点模式，避免 IP 变化
              </Typography>

              <Stack spacing={1}>
                <Alert severity="success">
                  <Typography variant="body2" fontWeight="bold">
                    流媒体服务（强制单节点）
                  </Typography>
                  <Typography variant="caption">
                    Netflix, YouTube, Hulu, Disney+, Prime Video
                  </Typography>
                </Alert>

                <Alert severity="success">
                  <Typography variant="body2" fontWeight="bold">
                    游戏服务（强制单节点）
                  </Typography>
                  <Typography variant="caption">
                    Steam, Epic Games, Riot Games, Blizzard
                  </Typography>
                </Alert>

                <Alert severity="info">
                  <Typography variant="body2" fontWeight="bold">
                    社交媒体（建议单节点）
                  </Typography>
                  <Typography variant="caption">
                    Twitter, Facebook, Instagram
                  </Typography>
                </Alert>

                <Alert severity="info">
                  <Typography variant="body2" fontWeight="bold">
                    下载服务（可多路径）
                  </Typography>
                  <Typography variant="caption">
                    GitHub, CDN 等
                  </Typography>
                </Alert>
              </Stack>
            </CardContent>
          </Card>
        </>
      )}
    </Box>
  )
}
