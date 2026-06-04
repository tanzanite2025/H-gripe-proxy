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
  rectSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useQuery } from '@tanstack/react-query'
import yaml from 'js-yaml'
import { Trash2, GripVertical, Link, Link2Off, AlertTriangle, HelpCircle, X, Building2, Settings } from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  closeAllConnections,
} from 'tauri-plugin-mihomo-api'

import { ResidentialPoolPanel } from '@/components/advanced/residential-pool-panel'
import { TooltipIcon } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { Dialog, DialogTitle, DialogContent, DialogActions } from '@/components/tailwind/Dialog'
import { IconButton } from '@/components/tailwind/IconButton'
import { Paper } from '@/components/tailwind/Paper'
import { useAppRefreshers, useProxiesData } from '@/providers/app-data-context'
import { getAdvancedConfig, saveAdvancedConfig, type ResidentialProxy } from '@/services/coordinator'
import {
  applyProxyRuntimeSelection,
  tryApplyProxyRuntimeSelection,
} from '@/services/proxy-runtime-selection'
import { debugLog } from '@/utils/misc'

import { ProxyChainHelpDialog } from './proxy-chain-help-dialog'
import {
  applyProxyChainRuntimeIntent,
  clearProxyChainRuntimeConfig,
} from './proxy-chain-runtime'
import {
  buildProxyChainRuntimeIntent,
  clearProxyChainStorage,
  isProxyChainConnected,
  loadProxyChainRuntimeGroup,
  proxyChainEntryNode,
  proxyChainTargetGroup,
  saveProxyChainRuntimeSelection,
  type ProxyChainItem,
} from './proxy-chain-types'

export type { ProxyChainItem } from './proxy-chain-types'

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
  bare?: boolean
  onClose?: () => void
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
    return 'border border-divider'
  }

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`mb-0 flex items-center p-2 rounded ${
        isDragging
          ? 'bg-background'
          : 'bg-card'
      } ${getBorderClass()} ${
        isDragging ? 'shadow-lg' : 'shadow'
      } transition-all duration-200`}
    >
      <div
        {...attributes}
        {...listeners}
        className="flex items-center mr-2 text-text-secondary cursor-grab active:cursor-grabbing"
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
  bare = false,
  onClose,
}: ProxyChainProps) => {
  const { t } = useTranslation()
  const chainWarning = t('proxies.page.chain.warning')
  const { proxies } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()

  const [isConnecting, setIsConnecting] = useState(false)
  const [helpDialogOpen, setHelpDialogOpen] = useState(false)
  const [residentialConfigOpen, setResidentialConfigOpen] = useState(false)
  const markUnsavedChanges = useCallback(() => {
    onMarkUnsavedChanges?.()
  }, [onMarkUnsavedChanges])

  // 住宅代理池数据
  const { data: advancedConfig } = useQuery({
    queryKey: ['advancedConfig'],
    queryFn: getAdvancedConfig,
    staleTime: 30_000,
  })
  const residentialPool = advancedConfig?.residential_pool
  const enabledResidentialProxies = residentialPool?.enabled
    ? residentialPool.proxies.filter((p: ResidentialProxy) => p.enabled)
    : []

  const [localResidentialPool, setLocalResidentialPool] = useState(
    residentialPool ?? { enabled: false, proxies: [] },
  )

  // 将住宅代理添加到链尾作为出口
  const addResidentialExit = useCallback(
    (proxy: ResidentialProxy) => {
      const resName = `VERGE-RES-${proxy.name}`
      if (proxyChain.some((item) => item.name === resName)) return
      const chainItem: ProxyChainItem = {
        id: `${resName}_${Date.now()}`,
        name: resName,
        type: proxy.proxyType,
      }
      onUpdateChain([...proxyChain, chainItem])
      markUnsavedChanges()
    },
    [proxyChain, onUpdateChain, markUnsavedChanges],
  )

  // 保存住宅代理池配置
  const handleSaveResidentialPool = useCallback(async () => {
    try {
      const fullConfig = await getAdvancedConfig()
      fullConfig.residential_pool = localResidentialPool
      await saveAdvancedConfig(fullConfig)
      setResidentialConfigOpen(false)
    } catch (error) {
      console.error('Failed to save residential pool config:', error)
    }
  }, [localResidentialPool])

  // 打开住宅代理池配置时同步最新数据
  const handleOpenResidentialConfig = useCallback(() => {
    if (residentialPool) {
      setLocalResidentialPool(residentialPool)
    }
    setResidentialConfigOpen(true)
  }, [residentialPool])

  const isConnected = useMemo(() => {
    return isProxyChainConnected(proxies, proxyChain, mode, selectedGroup)
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
        await clearProxyChainRuntimeConfig()

        const targetGroup = proxyChainTargetGroup(
          mode,
          selectedGroup,
          loadProxyChainRuntimeGroup(),
        )

        if (targetGroup) {
          const selectedDirect = await tryApplyProxyRuntimeSelection(
            targetGroup,
            'DIRECT',
          )
          if (!selectedDirect) {
            const entryNode = proxyChainEntryNode(proxyChain)
            if (entryNode) {
              await tryApplyProxyRuntimeSelection(targetGroup, entryNode.name)
            }
          }
        }

        clearProxyChainStorage()

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
      const intent = buildProxyChainRuntimeIntent(proxyChain, mode, selectedGroup)
      if (!intent) {
        throw new Error('invalid proxy chain intent')
      }

      debugLog('Saving chain config:', intent.runtimePayload)
      await applyProxyChainRuntimeIntent(intent)
      debugLog('Chain configuration saved successfully')

      debugLog(`Connecting to proxy chain, last node: ${intent.exitNode}`)

      await applyProxyRuntimeSelection(intent.targetGroup, intent.exitNode)
      saveProxyChainRuntimeSelection(intent.targetGroup, intent.exitNode)

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

  const Wrapper = bare ? 'div' : Paper
  const wrapperClassName = bare ? 'h-full p-4 flex flex-col' : 'h-full p-4 flex flex-col'

  return (
    <Wrapper className={wrapperClassName}>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <h3 className="text-lg font-semibold">{t('proxies.page.chain.header')}</h3>
          <span className="text-sm text-text-secondary">
            {proxyChain.length === 1
              ? t('proxies.page.chain.minimumNodesHint') ||
                '链式代理至少需要2个节点，请再添加一个节点。'
              : t('proxies.page.chain.instruction') ||
                '按顺序点击节点添加到代理链中'}
          </span>
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
                clearProxyChainRuntimeConfig()
                clearProxyChainStorage()
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
          <IconButton
            size="small"
            onClick={handleOpenResidentialConfig}
            title="住宅代理池配置"
          >
            <Settings className="h-4 w-4" />
          </IconButton>
          <Button
            size="small"
            variant={isConnected ? 'outlined' : 'primary'}
            startIcon={isConnected ? <Link2Off className="h-4 w-4" /> : <Link className="h-4 w-4" />}
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
          {onClose && (
            <IconButton size="small" onClick={onClose}>
              <X className="h-4 w-4" />
            </IconButton>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        {proxyChain.length === 0 ? (
          <div className="flex items-center justify-center h-full text-text-secondary">
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
              strategy={rectSortingStrategy}
            >
              <div className="grid grid-cols-4 gap-2 p-2">
                {proxyChain.map((proxy, index) => (
                  <SortableItem
                    key={proxy.id}
                    proxy={proxy}
                    index={index}
                    isFirst={index === 0}
                    isLast={
                      index === proxyChain.length - 1 && proxyChain.length > 1
                    }
                    onRemove={handleRemoveProxy}
                  />
                ))}
              </div>
            </SortableContext>
          </DndContext>
        )}
      </div>

      {/* 住宅代理出口选择 */}
      <div className="mt-2 px-2">
        <div className="flex items-center gap-1 mb-2">
          <Building2 className="h-3.5 w-3.5 text-secondary" />
          <span className="text-xs font-medium text-text-secondary">住宅代理出口</span>
        </div>
        {enabledResidentialProxies.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            {enabledResidentialProxies.map((proxy: ResidentialProxy) => {
              const resName = `VERGE-RES-${proxy.name}`
              const isSelected = proxyChain.some((item) => item.name === resName)
              return (
                <button
                  key={proxy.name}
                  onClick={() => addResidentialExit(proxy)}
                  disabled={isSelected}
                  className={`text-xs px-2 py-1 rounded border transition-colors ${
                    isSelected
                      ? 'border-orange-500/50 bg-orange-500/10 text-orange-400 cursor-default'
                      : 'border-divider bg-card hover:bg-primary/10 hover:border-primary hover:text-primary cursor-pointer'
                  }`}
                >
                  {proxy.name}
                  {isSelected && ' ✓'}
                </button>
              )
            })}
          </div>
        ) : (
          <div className="text-xs text-text-secondary/60 py-1">
            {residentialPool?.enabled
              ? '暂无已启用的住宅代理节点'
              : '住宅代理池未启用，启用后可选择住宅出口'}
          </div>
        )}
      </div>

      {/* 住宅代理池配置对话框 */}
      <Dialog open={residentialConfigOpen} onClose={() => setResidentialConfigOpen(false)}>
        <DialogTitle>住宅代理池配置</DialogTitle>
        <DialogContent>
          <ResidentialPoolPanel
            config={localResidentialPool}
            onChange={setLocalResidentialPool}
          />
        </DialogContent>
        <DialogActions>
          <Button variant="outlined" onClick={() => setResidentialConfigOpen(false)}>取消</Button>
          <Button onClick={handleSaveResidentialPool}>保存</Button>
        </DialogActions>
      </Dialog>

      {/* 帮助对话框 */}
      <ProxyChainHelpDialog
        open={helpDialogOpen}
        onClose={() => setHelpDialogOpen(false)}
      />
    </Wrapper>
  )
}
