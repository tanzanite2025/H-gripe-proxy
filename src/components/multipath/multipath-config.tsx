/**
 * 多路径路由配置组件
 */

import {
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  FormControlLabel,
  IconButton,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Stack,
  Switch,
  Tab,
  Tabs,
  TextField,
  Typography,
} from '@mui/material'
import {
  AddOutlined,
  DeleteOutlined,
  DownloadOutlined,
  InfoOutlined,
  RouteOutlined,
  UploadOutlined,
  WarningAmberOutlined,
} from '@mui/icons-material'
import { useState } from 'react'

import {
  type MultipathConfig,
  type NodePool,
  type PathNode,
  type PoolType,
  type SessionBinding,
  type SlicingStrategy,
  multipathAddBinding,
  multipathAddNode,
  multipathAddPool,
  multipathExportNodes,
  multipathGetBindings,
  multipathGetConfig,
  multipathGetPredefinedBindings,
  multipathGetRecommendedConfig,
  multipathImportNodes,
  multipathRemoveBinding,
  multipathRemoveNode,
  multipathRemovePool,
  multipathUpdateConfig,
  multipathUpdatePool,
} from '@/services/multipath'
import { showNotice } from '@/services/notice-service'
import { useMultiConfigLoader, useConfigSaver } from '@/hooks'

export default function MultipathConfig() {
  const [tabValue, setTabValue] = useState(0)
  const [poolDialogOpen, setPoolDialogOpen] = useState(false)
  const [nodeDialogOpen, setNodeDialogOpen] = useState(false)
  const [selectedPool, setSelectedPool] = useState<string>('')

  // 使用通用 Hook 加载多个配置
  const {
    data,
    loading,
    reload,
  } = useMultiConfigLoader({
    loaders: {
      config: multipathGetConfig,
      bindings: multipathGetBindings,
      predefinedBindings: multipathGetPredefinedBindings,
    },
  })

  // 解构配置数据
  const config = data?.config || null
  const bindings = data?.bindings || []
  const predefinedBindings = data?.predefinedBindings || []

  // 使用通用 Hook 保存配置
  const { save, saving } = useConfigSaver({
    saveFn: multipathUpdateConfig,
    onSuccess: reload,
    successMessage: '配置已保存',
  })

  // 本地配置状态（用于编辑）
  const [localConfig, setLocalConfig] = useState<MultipathConfig | null>(
    config || null
  )

  // 当配置加载完成时，更新本地配置
  useState(() => {
    if (config) {
      setLocalConfig(config)
    }
  })

  const handleSaveConfig = () => {
    if (!localConfig) return
    save(localConfig)
  }

  const handleLoadRecommended = async () => {
    try {
      const recommended = await multipathGetRecommendedConfig()
      setLocalConfig(recommended)
      showNotice('success', '已加载推荐配置')
    } catch (error: any) {
      showNotice('error', `加载失败: ${error.message || error}`)
    }
  }

  const handleAddPool = async (pool: NodePool) => {
    try {
      await multipathAddPool(pool)
      await reload()
      setPoolDialogOpen(false)
      showNotice('success', '节点池已添加')
    } catch (error: any) {
      showNotice('error', `添加失败: ${error.message || error}`)
    }
  }

  const handleRemovePool = async (poolName: string) => {
    try {
      await multipathRemovePool(poolName)
      await reload()
      showNotice('success', '节点池已删除')
    } catch (error: any) {
      showNotice('error', `删除失败: ${error.message || error}`)
    }
  }

  const handleAddNode = async (poolName: string, node: PathNode) => {
    try {
      await multipathAddNode(poolName, node)
      await reload()
      setNodeDialogOpen(false)
      showNotice('success', '节点已添加')
    } catch (error: any) {
      showNotice('error', `添加失败: ${error.message || error}`)
    }
  }

  const handleRemoveNode = async (poolName: string, nodeName: string) => {
    try {
      await multipathRemoveNode(poolName, nodeName)
      await reload()
      showNotice('success', '节点已删除')
    } catch (error: any) {
      showNotice('error', `删除失败: ${error.message || error}`)
    }
  }

  const handleExportNodes = async (poolName: string) => {
    try {
      const yaml = await multipathExportNodes(poolName)
      const blob = new Blob([yaml], { type: 'text/yaml' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `${poolName}-nodes.yaml`
      a.click()
      URL.revokeObjectURL(url)
      showNotice('success', '节点已导出')
    } catch (error: any) {
      showNotice('error', `导出失败: ${error.message || error}`)
    }
  }

  const handleImportNodes = async (poolName: string, file: File) => {
    try {
      const yaml = await file.text()
      const result = await multipathImportNodes(poolName, yaml)
      await reload()
      showNotice('success', result.message)
    } catch (error: any) {
      showNotice('error', `导入失败: ${error.message || error}`)
    }
  }

  const getPoolTypeLabel = (type: PoolType) => {
    const labels: Record<PoolType, string> = {
      General: '通用',
      Streaming: '流媒体',
      Gaming: '游戏',
      Download: '下载',
      Social: '社交',
    }
    return labels[type]
  }

  const getStrategyLabel = (strategy: SlicingStrategy) => {
    const labels: Record<SlicingStrategy, string> = {
      RoundRobin: '轮询',
      Random: '随机',
      Weighted: '加权',
      LeastConnections: '最少连接',
      LatencyBased: '延迟优先',
    }
    return labels[strategy]
  }

  if (loading || !localConfig) {
    return <Box sx={{ p: 3 }}>加载中...</Box>
  }

  return (
    <Box sx={{ p: 3 }}>
      <Stack spacing={3}>
        {/* 标题 */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <RouteOutlined color="primary" />
          <Typography variant="h6">多路径阴影路由</Typography>
        </Box>

        {/* 警告说明 */}
        <Paper
          sx={{ p: 2, bgcolor: 'warning.main', color: 'warning.contrastText' }}
        >
          <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
            <WarningAmberOutlined />
            <Box>
              <Typography variant="body2" sx={{ fontWeight: 600 }}>
                重要：避免 IP 乱跳导致封号
              </Typography>
              <Typography variant="caption">
                流媒体（Netflix、YouTube）、游戏、社交媒体等服务必须使用单节点模式，否则会因
                IP 变化被封号。系统已预配置安全规则。
              </Typography>
            </Box>
          </Box>
        </Paper>

        {/* 标签页 */}
        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Tabs value={tabValue} onChange={(_, v) => setTabValue(v)}>
            <Tab label="基础配置" />
            <Tab label="节点池管理" />
            <Tab label="会话绑定规则" />
          </Tabs>
        </Box>

        {/* 基础配置 */}
        {tabValue === 0 && (
          <Stack spacing={2}>
            <Card>
              <CardContent>
                <Stack spacing={2}>
                  <FormControlLabel
                    control={
                      <Switch
                        checked={localConfig.enabled}
                        onChange={(e) =>
                          setLocalConfig({ ...localConfig, enabled: e.target.checked })
                        }
                      />
                    }
                    label="启用多路径路由"
                  />

                  <FormControl fullWidth>
                    <InputLabel>分片策略</InputLabel>
                    <Select
                      value={localConfig.strategy}
                      label="分片策略"
                      onChange={(e) =>
                        setLocalConfig({
                          ...localConfig,
                          strategy: e.target.value as SlicingStrategy,
                        })
                      }
                      disabled={!localConfig.enabled}
                    >
                      <MenuItem value="RoundRobin">轮询（均匀分配）</MenuItem>
                      <MenuItem value="Random">随机</MenuItem>
                      <MenuItem value="Weighted">加权（推荐）</MenuItem>
                      <MenuItem value="LeastConnections">最少连接</MenuItem>
                      <MenuItem value="LatencyBased">延迟优先</MenuItem>
                    </Select>
                  </FormControl>

                  <TextField
                    label="最小分片大小（字节）"
                    type="number"
                    value={localConfig.min_fragment_size}
                    onChange={(e) =>
                      setLocalConfig({
                        ...localConfig,
                        min_fragment_size: Number.parseInt(e.target.value),
                      })
                    }
                    disabled={!localConfig.enabled}
                    fullWidth
                  />

                  <TextField
                    label="最大分片大小（字节）"
                    type="number"
                    value={localConfig.max_fragment_size}
                    onChange={(e) =>
                      setLocalConfig({
                        ...localConfig,
                        max_fragment_size: Number.parseInt(e.target.value),
                      })
                    }
                    disabled={!localConfig.enabled}
                    fullWidth
                  />

                  <TextField
                    label="重组超时（毫秒）"
                    type="number"
                    value={localConfig.reassembly_timeout}
                    onChange={(e) =>
                      setLocalConfig({
                        ...localConfig,
                        reassembly_timeout: Number.parseInt(e.target.value),
                      })
                    }
                    disabled={!localConfig.enabled}
                    fullWidth
                  />

                  <FormControlLabel
                    control={
                      <Switch
                        checked={localConfig.session_persistence}
                        onChange={(e) =>
                          setLocalConfig({
                            ...localConfig,
                            session_persistence: e.target.checked,
                          })
                        }
                        disabled={!localConfig.enabled}
                      />
                    }
                    label="启用会话保持"
                  />
                </Stack>
              </CardContent>
            </Card>

            <Paper sx={{ p: 2, bgcolor: 'info.main', color: 'info.contrastText' }}>
              <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
                <InfoOutlined />
                <Box>
                  <Typography variant="body2" sx={{ fontWeight: 600 }}>
                    工作原理
                  </Typography>
                  <Typography variant="caption">
                    将数据流切分成小片段，通过不同节点传输，在本地重组。单一路径上的审查设备只能看到残缺的加密碎片，无法进行行为分析。
                  </Typography>
                </Box>
              </Box>
            </Paper>

            <Box sx={{ display: 'flex', gap: 2 }}>
              <Button
                variant="contained"
                onClick={handleSaveConfig}
                disabled={saving || loading}
                fullWidth
              >
                保存配置
              </Button>
              <Button
                variant="outlined"
                onClick={handleLoadRecommended}
                disabled={saving || loading}
              >
                加载推荐配置
              </Button>
            </Box>
          </Stack>
        )}

        {/* 节点池管理 */}
        {tabValue === 1 && (
          <Stack spacing={2}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="subtitle2">节点池列表</Typography>
              <Button
                startIcon={<AddOutlined />}
                onClick={() => setPoolDialogOpen(true)}
              >
                添加节点池
              </Button>
            </Box>

            {localConfig.node_pools.map((pool) => (
              <Card key={pool.name}>
                <CardContent>
                  <Box
                    sx={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      mb: 2,
                    }}
                  >
                    <Box>
                      <Typography variant="subtitle1">{pool.name}</Typography>
                      <Box sx={{ display: 'flex', gap: 1, mt: 1 }}>
                        <Chip
                          label={getPoolTypeLabel(pool.pool_type)}
                          size="small"
                        />
                        <Chip
                          label={pool.enabled ? '已启用' : '已禁用'}
                          size="small"
                          color={pool.enabled ? 'success' : 'default'}
                        />
                        <Chip
                          label={`${pool.nodes.length} 个节点`}
                          size="small"
                        />
                      </Box>
                    </Box>
                    <Box>
                      <IconButton
                        onClick={() => handleExportNodes(pool.name)}
                        title="导出节点"
                      >
                        <DownloadOutlined />
                      </IconButton>
                      <IconButton
                        onClick={() => {
                          setSelectedPool(pool.name)
                          setNodeDialogOpen(true)
                        }}
                        title="添加节点"
                      >
                        <AddOutlined />
                      </IconButton>
                      <IconButton
                        onClick={() => handleRemovePool(pool.name)}
                        color="error"
                        title="删除节点池"
                      >
                        <DeleteOutlined />
                      </IconButton>
                    </Box>
                  </Box>

                  {pool.nodes.length > 0 && (
                    <Box>
                      <Typography variant="caption" color="text.secondary">
                        节点列表
                      </Typography>
                      <Stack spacing={1} sx={{ mt: 1 }}>
                        {pool.nodes.map((node) => (
                          <Box
                            key={node.name}
                            sx={{
                              display: 'flex',
                              justifyContent: 'space-between',
                              alignItems: 'center',
                              p: 1,
                              bgcolor: 'background.default',
                              borderRadius: 1,
                            }}
                          >
                            <Box>
                              <Typography variant="body2">
                                {node.name}
                              </Typography>
                              <Typography variant="caption" color="text.secondary">
                                {node.server}:{node.port} | 权重: {node.weight}
                              </Typography>
                            </Box>
                            <IconButton
                              size="small"
                              onClick={() =>
                                handleRemoveNode(pool.name, node.name)
                              }
                              color="error"
                            >
                              <DeleteOutlined fontSize="small" />
                            </IconButton>
                          </Box>
                        ))}
                      </Stack>
                    </Box>
                  )}
                </CardContent>
              </Card>
            ))}
          </Stack>
        )}

        {/* 会话绑定规则 */}
        {tabValue === 2 && (
          <Stack spacing={2}>
            <Paper sx={{ p: 2, bgcolor: 'error.main', color: 'error.contrastText' }}>
              <Typography variant="body2" sx={{ fontWeight: 600 }}>
                ⚠️ 以下服务必须使用单节点，否则会被封号
              </Typography>
            </Paper>

            <Typography variant="subtitle2">预定义规则（推荐）</Typography>
            {predefinedBindings.map((binding) => (
              <Card key={binding.domain_pattern}>
                <CardContent>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                    <Box>
                      <Typography variant="body2">
                        {binding.domain_pattern}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        {binding.description}
                      </Typography>
                      <Box sx={{ display: 'flex', gap: 1, mt: 1 }}>
                        <Chip
                          label={getPoolTypeLabel(binding.pool_type)}
                          size="small"
                        />
                        {binding.force_single_node && (
                          <Chip
                            label="强制单节点"
                            size="small"
                            color="error"
                          />
                        )}
                      </Box>
                    </Box>
                  </Box>
                </CardContent>
              </Card>
            ))}
          </Stack>
        )}
      </Stack>
    </Box>
  )
}
