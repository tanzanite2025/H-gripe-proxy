import {
  closestCenter,
  DndContext,
  type DragEndEvent,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import {
  arrayMove,
  rectSortingStrategy,
  SortableContext,
  sortableKeyboardCoordinates,
} from '@dnd-kit/sortable'
import {
  AlertTriangle,
  HelpCircle,
  Link,
  Link2Off,
  Settings,
  Trash2,
  X,
} from 'lucide-react'
import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { closeAllConnections } from 'tauri-plugin-mihomo-api'

import { TooltipIcon } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import { IconButton } from '@/components/tailwind/IconButton'
import { Paper } from '@/components/tailwind/Paper'
import { useAppRefreshers, useProxiesData } from '@/providers/app-data-context'
import { applyProxyRuntimeSelection, tryApplyProxyRuntimeSelection } from '@/services/proxy-runtime-selection'
import { debugLog } from '@/utils/misc'

import { ProxyChainHelpDialog } from '../proxy-chain-help-dialog'
import {
  applyProxyChainRuntimeIntent,
  clearProxyChainRuntimeConfig,
} from '../proxy-chain-runtime'
import {
  buildProxyChainRuntimeIntent,
  clearProxyChainStorage,
  isProxyChainConnected,
  loadProxyChainRuntimeGroup,
  proxyChainEntryNode,
  proxyChainTargetGroup,
  saveProxyChainRuntimeSelection,
  type ProxyChainItem,
} from '../proxy-chain-types'
import { ResidentialExitSection } from './residential-exit-section'
import { ResidentialPoolDialog } from './residential-pool-dialog'
import { SortableChainItem } from './sortable-chain-item'
import {
  useProxyChainConfigLoader,
  useProxyChainDelayUpdater,
  useProxyChainLengthDirtyMarker,
  useResidentialPoolState,
} from './use-proxy-chain-side-effects'

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

const getTranslatedLabel = (
  t: (key: string) => string,
  key: string,
  fallback: string,
) => {
  const translated = t(key)
  return !translated || translated === key ? fallback : translated
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
  const { proxies } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const [isConnecting, setIsConnecting] = useState(false)
  const [helpDialogOpen, setHelpDialogOpen] = useState(false)

  const markUnsavedChanges = useCallback(() => {
    onMarkUnsavedChanges?.()
  }, [onMarkUnsavedChanges])

  const {
    residentialPool,
    enabledResidentialProxies,
    localResidentialPool,
    residentialConfigOpen,
    setLocalResidentialPool,
    setResidentialConfigOpen,
    addResidentialExit,
    openResidentialConfig,
    saveResidentialPool,
  } = useResidentialPoolState(proxyChain, onUpdateChain, markUnsavedChanges)

  useProxyChainLengthDirtyMarker(proxyChain.length, markUnsavedChanges)
  useProxyChainConfigLoader(chainConfigData, onUpdateChain)
  useProxyChainDelayUpdater(proxies?.records, proxyChain, onUpdateChain)

  const isConnected = useMemo(
    () => isProxyChainConnected(proxies, proxyChain, mode, selectedGroup),
    [proxies, proxyChain, mode, selectedGroup],
  )

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )

  const entryLabel = getTranslatedLabel(
    t,
    'proxies.page.chain.entryNode',
    '入口节点',
  )
  const exitLabel = getTranslatedLabel(
    t,
    'proxies.page.chain.exitNode',
    '出口节点',
  )
  const timeoutLabel = getTranslatedLabel(t, 'shared.labels.timeout', '超时')
  const chainHeader = getTranslatedLabel(
    t,
    'proxies.page.chain.header',
    '代理链',
  )
  const chainWarning = getTranslatedLabel(
    t,
    'proxies.page.chain.warning',
    '代理链会增加延迟，并且整条链的稳定性取决于最慢或最不稳定的节点。',
  )
  const chainInstruction =
    proxyChain.length === 1
      ? getTranslatedLabel(
          t,
          'proxies.page.chain.minimumNodesHint',
          '代理链至少需要 2 个节点，请再添加一个节点。',
        )
      : getTranslatedLabel(
          t,
          'proxies.page.chain.instruction',
          '按顺序点击节点，把它们添加到代理链里。',
        )
  const minimumNodesMessage = getTranslatedLabel(
    t,
    'proxies.page.chain.minimumNodes',
    '代理链至少需要 2 个节点',
  )
  const disconnectFailedMessage = getTranslatedLabel(
    t,
    'proxies.page.chain.disconnectFailed',
    '断开代理链失败',
  )
  const connectFailedMessage = getTranslatedLabel(
    t,
    'proxies.page.chain.connectFailed',
    '连接代理链失败',
  )
  const clearChainLabel = getTranslatedLabel(
    t,
    'proxies.page.actions.clearChainConfig',
    '删除链式配置',
  )
  const connectingLabel = getTranslatedLabel(
    t,
    'proxies.page.actions.connecting',
    '连接中...',
  )
  const disconnectLabel = getTranslatedLabel(
    t,
    'proxies.page.actions.disconnect',
    '断开',
  )
  const connectLabel = getTranslatedLabel(
    t,
    'proxies.page.actions.connect',
    '连接',
  )
  const emptyLabel = getTranslatedLabel(
    t,
    'proxies.page.chain.empty',
    '暂时还没有添加节点',
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
      onUpdateChain(proxyChain.filter((item) => item.id !== id))
      markUnsavedChanges()
    },
    [proxyChain, onUpdateChain, markUnsavedChanges],
  )

  const handleClearChain = useCallback(() => {
    void clearProxyChainRuntimeConfig()
    clearProxyChainStorage()
    onUpdateChain([])
  }, [onUpdateChain])

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
        alert(disconnectFailedMessage)
      } finally {
        setIsConnecting(false)
      }
      return
    }

    if (proxyChain.length < 2) {
      alert(minimumNodesMessage)
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

      void refreshProxy()
      debugLog('Successfully connected to proxy chain')
    } catch (error) {
      console.error('Failed to connect to proxy chain:', error)
      alert(connectFailedMessage)
    } finally {
      setIsConnecting(false)
    }
  }, [
    connectFailedMessage,
    disconnectFailedMessage,
    isConnected,
    minimumNodesMessage,
    mode,
    onUpdateChain,
    proxyChain,
    refreshProxy,
    selectedGroup,
  ])

  const WrapperComponent = bare ? 'div' : Paper

  return (
    <WrapperComponent className="flex h-full flex-col p-4">
      <div className="mb-4 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="text-lg font-semibold">{chainHeader}</h3>
          <span className="text-sm text-text-secondary">{chainInstruction}</span>
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
              onClick={handleClearChain}
              className="text-red-500 hover:bg-red-500/10"
              title={clearChainLabel}
            >
              <Trash2 className="h-4 w-4" />
            </IconButton>
          )}

          <IconButton
            size="small"
            onClick={openResidentialConfig}
            title="住宅代理池配置"
          >
            <Settings className="h-4 w-4" />
          </IconButton>

          <Button
            size="small"
            variant={isConnected ? 'outlined' : 'primary'}
            startIcon={
              isConnected ? (
                <Link2Off className="h-4 w-4" />
              ) : (
                <Link className="h-4 w-4" />
              )
            }
            onClick={handleConnect}
            disabled={
              isConnecting ||
              proxyChain.length < 2 ||
              (mode !== 'global' && !selectedGroup)
            }
            className={`min-w-[90px] ${
              isConnected ? 'border-red-500 text-red-500 hover:bg-red-500/10' : ''
            }`}
            title={proxyChain.length < 2 ? minimumNodesMessage : undefined}
          >
            {isConnecting
              ? connectingLabel
              : isConnected
                ? disconnectLabel
                : connectLabel}
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
          <div className="flex h-full items-center justify-center text-text-secondary">
            <span>{emptyLabel}</span>
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
                  <SortableChainItem
                    key={proxy.id}
                    proxy={proxy}
                    index={index}
                    isFirst={index === 0}
                    isLast={index === proxyChain.length - 1 && proxyChain.length > 1}
                    entryLabel={entryLabel}
                    exitLabel={exitLabel}
                    timeoutLabel={timeoutLabel}
                    onRemove={handleRemoveProxy}
                  />
                ))}
              </div>
            </SortableContext>
          </DndContext>
        )}
      </div>

      <ResidentialExitSection
        proxyChain={proxyChain}
        residentialPool={residentialPool}
        enabledResidentialProxies={enabledResidentialProxies}
        onAddResidentialExit={addResidentialExit}
      />

      <ResidentialPoolDialog
        open={residentialConfigOpen}
        config={localResidentialPool}
        onChange={setLocalResidentialPool}
        onClose={() => setResidentialConfigOpen(false)}
        onSave={saveResidentialPool}
      />

      <ProxyChainHelpDialog
        open={helpDialogOpen}
        onClose={() => setHelpDialogOpen(false)}
      />
    </WrapperComponent>
  )
}
