import {
  closestCenter,
  DndContext,
  DragEndEvent,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { ArrowDown, Trash2, GripVertical, Link, LinkOff, AlertTriangle, HelpCircle } from 'lucide-react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { IconButton } from '@/components/tailwind/IconButton'
import { Paper } from '@/components/tailwind/Paper'
import { useThemeMode } from '@/services/states'
import yaml from 'js-yaml'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  closeAllConnections,
  selectNodeForGroup,
} from 'tauri-plugin-mihomo-api'

import { TooltipIcon } from '@/components/base'
import { useAppRefreshers, useProxiesData } from '@/providers/app-data-context'
import { updateProxyChainConfigInRuntime } from '@/services/cmds'
import { debugLog } from '@/utils/misc'

import { ProxyChainHelpDialog } from './proxy-chain-help-dialog'

interface ProxyChainItem {
  id: string
  name: string
  type?: string
  delay?: number
}

interface ParsedChainConfig {
  proxies?: Array<{
    name: string
    type: string
    [key: string]: any
  }>
}

interface ProxyChainProps {
  proxyChain: ProxyChainItem[]
  onUpdateChain: (chain: ProxyChainItem[]) => void
  chainConfigData?: string | null
  onMarkUnsavedChanges?: () => void
  mode?: string
  selectedGroup?: string | null
}

interface SortableItemProps {
  proxy: ProxyChainItem
  index: number
  isFirst: boolean
  isLast: boolean
  onRemove: (id: string) => void
}

const toChainItems = (
  parsedConfig: ParsedChainConfig | null | undefined,
): ProxyChainItem[] => {
  const timestamp = Date.now()

  return (
    parsedConfig?.proxies?.map((proxy, index) => ({
      id: `${proxy.name}_${timestamp}_${index}`,
      name: proxy.name,
      type: proxy.type,
      delay: undefined,
    })) || []
  )
}

const SortableItem = ({
  proxy,
  index,
  isFirst,
  isLast,
  onRemove,
}: SortableItemProps) => {
  const { t } = useTranslation()
  const mode = useThemeMode()
  const isDark = mode === 'dark'
  
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: proxy.id })

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  }

  const roleLabel = isFirst
    ? t('proxies.page.chain.entryNode')
    : isLast
      ? t('proxies.page.chain.exitNode')
      : undefined

  const getBorderClass = () => {
    if (isFirst) return 'border-2 border-green-500'
    if (isLast) return 'border-2 border-orange-500'
    return 'border border-gray-300 dark:border-gray-700'
  }

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`mb-0 flex items-center p-2 rounded ${
        isDragging
          ? 'bg-gray-100 dark:bg-gray-800'
          : 'bg-white dark:bg-gray-900'
      } ${getBorderClass()} ${
        isDragging ? 'shadow-lg' : 'shadow'
      } transition-all duration-200`}
    >
      <div
        {...attributes}
        {...listeners}
        className="flex items-center mr-2 text-gray-500 cursor-grab active:cursor-grabbing"
      >
        <GripVertical className="h-5 w-5" />
      </div>

      {roleLabel ? (
        <Chip
          label={roleLabel}
          size="small"
          className={`mr-2 font-bold text-white ${
            isFirst ? 'bg-green-500' : 'bg-orange-500'
          }`}
        />
      ) : (
        <Chip
          label={`${index + 1}`}
          size="small"
          color="primary"
          className="mr-2 min-w-[32px]"
        />
      )}

      <span className="flex-1 text-sm font-medium overflow-hidden text-ellipsis whitespace-nowrap">
        {proxy.name}
      </span>

      {proxy.type && (
        <Chip
          label={proxy.type}
          size="small"
          variant="outlined"
          className="mr-2"
        />
      )}

      {proxy.delay !== undefined && (
        <Chip
          label={
            proxy.delay > 0
              ? `${proxy.delay}ms`
              : t('shared.labels.timeout') || '超时'
          }
          size="small"
          color={
            proxy.delay > 0 && proxy.delay < 200
              ? 'success'
              : proxy.delay > 0 && proxy.delay < 800
                ? 'warning'
                : 'error'
          }
          className="mr-2 text-xs min-w-[50px]"
        />
      )}

      <IconButton
        size="small"
        onClick={() => onRemove(proxy.id)}
        className="text-red-500 hover:bg-red-500/10"
      >
        <Trash2 className="h-4 w-4" />
      </IconButton>
    </div>
  )
}

export const ProxyChain = ({
  proxyChain,
  onUpdateChain,
  chainConfigData,
  onMarkUnsavedChanges,
  mode,
  selectedGroup,
}: ProxyChainProps) => {
  const { t } = useTranslation()
  const themeMode = useThemeMode()
  const chainWarning = t('proxies.page.chain.warning')
  const { proxies } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const [isConnecting, setIsConnecting] = useState(false)
  const [helpDialogOpen, setHelpDialogOpen] = useState(false)
  const markUnsavedChanges = useCallback(() => {
    onMarkUnsavedChanges?.()
  }, [onMarkUnsavedChanges])

  const isConnected = useMemo(() => {
    if (!proxies || proxyChain.length < 2) {
      return false
    }

    const lastNode = proxyChain[proxyChain.length - 1]

    if (mode === 'global') {
      return proxies.global?.now === lastNode.name
    }

    if (!selectedGroup || !Array.isArray(proxies.groups)) {
      return false
    }

    const proxyChainGroup = proxies.groups.find(
      (group: { name: string }) => group.name === selectedGroup,
    )

    return proxyChainGroup?.now === lastNode.name
  }, [proxies, proxyChain, mode, selectedGroup])

  // 监听链的变化，但排除从配置加载的情况
  const chainLengthRef = useRef(proxyChain.length)
  useEffect(() => {
    // 只有当链长度发生变化且不是初始加载时，才标记为未保存
    if (
      chainLengthRef.current !== proxyChain.length &&
      chainLengthRef.current !== 0
    ) {
      markUnsavedChanges()
    }
    chainLengthRef.current = proxyChain.length
  }, [proxyChain.length, markUnsavedChanges])

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event

      if (active.id !== over?.id) {
        const oldIndex = proxyChain.findIndex((item) => item.id === active.id)
        const newIndex = proxyChain.findIndex((item) => item.id === over?.id)

        onUpdateChain(arrayMove(proxyChain, oldIndex, newIndex))
        markUnsavedChanges()
      }
    },
    [proxyChain, onUpdateChain, markUnsavedChanges],
  )

  const handleRemoveProxy = useCallback(
    (id: string) => {
      const newChain = proxyChain.filter((item) => item.id !== id)
      onUpdateChain(newChain)
      markUnsavedChanges()
    },
    [proxyChain, onUpdateChain, markUnsavedChanges],
  )

  const handleConnect = useCallback(async () => {
    if (isConnected) {
      setIsConnecting(true)
      try {
        await updateProxyChainConfigInRuntime(null)

        const targetGroup =
          mode === 'global'
            ? 'GLOBAL'
            : selectedGroup || localStorage.getItem('proxy-chain-group')

        if (targetGroup) {
          try {
            await selectNodeForGroup(targetGroup, 'DIRECT')
          } catch {
            if (proxyChain.length >= 1) {
              try {
                await selectNodeForGroup(targetGroup, proxyChain[0].name)
              } catch {
                // ignore
              }
            }
          }
        }

        localStorage.removeItem('proxy-chain-group')
        localStorage.removeItem('proxy-chain-exit-node')
        localStorage.removeItem('proxy-chain-items')

        await closeAllConnections()
        await refreshProxy()

        onUpdateChain([])
      } catch (error) {
        console.error('Failed to disconnect from proxy chain:', error)
        alert(t('proxies.page.chain.disconnectFailed') || '断开链式代理失败')
      } finally {
        setIsConnecting(false)
      }
      return
    }

    if (proxyChain.length < 2) {
      alert(t('proxies.page.chain.minimumNodes') || '链式代理至少需要2个节点')
      return
    }

    setIsConnecting(true)
    try {
      // 第一步：保存链式代理配置
      const chainProxies = proxyChain.map((node) => node.name)
      debugLog('Saving chain config:', chainProxies)
      await updateProxyChainConfigInRuntime(chainProxies)
      debugLog('Chain configuration saved successfully')

      // 第二步：连接到代理链的最后一个节点
      const lastNode = proxyChain[proxyChain.length - 1]
      debugLog(`Connecting to proxy chain, last node: ${lastNode.name}`)

      // 根据模式确定使用的代理组名称
      if (mode !== 'global' && !selectedGroup) {
        throw new Error('规则模式下必须选择代理组')
      }

      const targetGroup = mode === 'global' ? 'GLOBAL' : selectedGroup

      await selectNodeForGroup(targetGroup || 'GLOBAL', lastNode.name)
      localStorage.setItem('proxy-chain-group', targetGroup || 'GLOBAL')
      localStorage.setItem('proxy-chain-exit-node', lastNode.name)

      // 刷新代理信息以更新连接状态
      refreshProxy()
      debugLog('Successfully connected to proxy chain')
    } catch (error) {
      console.error('Failed to connect to proxy chain:', error)
      alert(t('proxies.page.chain.connectFailed') || '连接链式代理失败')
    } finally {
      setIsConnecting(false)
    }
  }, [
    proxyChain,
    isConnected,
    t,
    refreshProxy,
    mode,
    selectedGroup,
    onUpdateChain,
  ])

  const proxyChainRef = useRef(proxyChain)
  const onUpdateChainRef = useRef(onUpdateChain)

  useEffect(() => {
    proxyChainRef.current = proxyChain
    onUpdateChainRef.current = onUpdateChain
  }, [proxyChain, onUpdateChain])

  // 处理链式代理配置数据
  useEffect(() => {
    if (chainConfigData) {
      try {
        // JSON is valid YAML, so one parser covers both persisted formats.
        const parsedConfig = yaml.load(chainConfigData) as ParsedChainConfig
        const chainItems = toChainItems(parsedConfig)

        if (chainItems.length > 0) {
          onUpdateChain(chainItems)
        }
      } catch (error) {
        console.error('Failed to process chain config data:', error)
      }
    }
  }, [chainConfigData, onUpdateChain])

  // 定时更新延迟数据
  useEffect(() => {
    if (!proxies?.records) return

    const updateDelays = () => {
      const currentChain = proxyChainRef.current
      if (currentChain.length === 0) return

      const updatedChain = currentChain.map((item) => {
        const proxyRecord = proxies.records[item.name]
        if (
          proxyRecord &&
          proxyRecord.history &&
          proxyRecord.history.length > 0
        ) {
          const latestDelay =
            proxyRecord.history[proxyRecord.history.length - 1].delay
          return { ...item, delay: latestDelay }
        }
        return item
      })

      // 只有在延迟数据确实发生变化时才更新
      const hasChanged = updatedChain.some(
        (item, index) => item.delay !== currentChain[index]?.delay,
      )

      if (hasChanged) {
        onUpdateChainRef.current(updatedChain)
      }
    }

    // 立即更新一次延迟
    updateDelays()

    // 设置定时器，每5秒更新一次延迟
    const interval = setInterval(updateDelays, 5000)

    return () => clearInterval(interval)
  }, [proxies?.records]) // 只依赖proxies.records

  return (
    <Paper className="h-full p-4 flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <h3 className="text-lg font-semibold">{t('proxies.page.chain.header')}</h3>
          <TooltipIcon
            title={chainWarning}
            icon={AlertTriangle}
            color="warning"
            className="p-1"
          />
          <IconButton
            size="small"
            onClick={() => setHelpDialogOpen(true)}
            className="ml-1"
            title="使用帮助"
          >
            <HelpCircle className="h-4 w-4" />
          </IconButton>
        </div>
        <div className="flex items-center gap-2">
          {proxyChain.length > 0 && (
            <IconButton
              size="small"
              onClick={() => {
                updateProxyChainConfigInRuntime(null)
                localStorage.removeItem('proxy-chain-group')
                localStorage.removeItem('proxy-chain-exit-node')
                localStorage.removeItem('proxy-chain-items')
                onUpdateChain([])
              }}
              className="text-red-500 hover:bg-red-500/10"
              title={
                t('proxies.page.actions.clearChainConfig') || '删除链式配置'
              }
            >
              <Trash2 className="h-4 w-4" />
            </IconButton>
          )}
          <Button
            size="small"
            variant={isConnected ? 'outlined' : 'primary'}
            startIcon={isConnected ? <LinkOff className="h-4 w-4" /> : <Link className="h-4 w-4" />}
            onClick={handleConnect}
            disabled={
              isConnecting ||
              proxyChain.length < 2 ||
              (mode !== 'global' && !selectedGroup)
            }
            className={`min-w-[90px] ${
              isConnected ? 'text-red-500 border-red-500 hover:bg-red-500/10' : ''
            }`}
            title={
              proxyChain.length < 2
                ? t('proxies.page.chain.minimumNodes') ||
                  '链式代理至少需要2个节点'
                : undefined
            }
          >
            {isConnecting
              ? t('proxies.page.actions.connecting') || '连接中...'
              : isConnected
                ? t('proxies.page.actions.disconnect') || '断开'
                : t('proxies.page.actions.connect') || '连接'}
          </Button>
        </div>
      </div>

      <Alert
        severity={proxyChain.length === 1 ? 'warning' : 'info'}
        className="mb-4"
      >
        {proxyChain.length === 1
          ? t('proxies.page.chain.minimumNodesHint') ||
            '链式代理至少需要2个节点，请再添加一个节点。'
          : t('proxies.page.chain.instruction') ||
            '按顺序点击节点添加到代理链中'}
      </Alert>

      <div className="flex-1 overflow-auto">
        {proxyChain.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-500">
            <span>{t('proxies.page.chain.empty')}</span>
          </div>
        ) : (
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragEnd={handleDragEnd}
          >
            <SortableContext
              items={proxyChain.map((proxy) => proxy.id)}
              strategy={verticalListSortingStrategy}
            >
              <div className="rounded min-h-[60px] p-2">
                {proxyChain.map((proxy, index) => (
                  <div key={proxy.id}>
                    <SortableItem
                      proxy={proxy}
                      index={index}
                      isFirst={index === 0}
                      isLast={
                        index === proxyChain.length - 1 && proxyChain.length > 1
                      }
                      onRemove={handleRemoveProxy}
                    />
                    {index < proxyChain.length - 1 && (
                      <div className="flex justify-center py-1">
                        <ArrowDown className="h-5 w-5 text-primary opacity-70" />
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </SortableContext>
          </DndContext>
        )}
      </div>

      {/* 帮助对话框 */}
      <ProxyChainHelpDialog
        open={helpDialogOpen}
        onClose={() => setHelpDialogOpen(false)}
      />
    </Paper>
  )
}
