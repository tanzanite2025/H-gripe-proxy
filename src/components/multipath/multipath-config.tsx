/**
 * 多路径路由配置组件
 */

import { useState } from 'react'

import { useMultiConfigLoader, useConfigSaver } from '@/hooks'
import {
  type MultipathConfig,
  type NodePool,
  type PathNode,
  type PoolType,
  type SlicingStrategy,
  multipathAddNode,
  multipathAddPool,
  multipathExportNodes,
  multipathGetBindings,
  multipathGetConfig,
  multipathGetPredefinedBindings,
  multipathGetRecommendedConfig,
  multipathImportNodes,
  multipathRemoveNode,
  multipathRemovePool,
  multipathUpdateConfig,
} from '@/services/multipath'
import { showNotice } from '@/services/notice-service'

import MultipathConfigUI from './multipath-config-ui'

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
    return <div className="p-6">加载中...</div>
  }

  return (
    <MultipathConfigUI
      config={localConfig}
      bindings={bindings}
      predefinedBindings={predefinedBindings}
      tabValue={tabValue}
      saving={saving}
      loading={loading}
      onConfigChange={setLocalConfig}
      onTabChange={setTabValue}
      onSaveConfig={handleSaveConfig}
      onLoadRecommended={handleLoadRecommended}
      onAddPool={() => setPoolDialogOpen(true)}
      onRemovePool={handleRemovePool}
      onExportNodes={handleExportNodes}
      onAddNode={(poolName) => {
        setSelectedPool(poolName)
        setNodeDialogOpen(true)
      }}
      onRemoveNode={handleRemoveNode}
      getPoolTypeLabel={getPoolTypeLabel}
      getStrategyLabel={getStrategyLabel}
    />
  )
}
