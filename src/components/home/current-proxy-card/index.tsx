import { EnhancedCard } from '@/components/home/enhanced-card'

import { CurrentProxyCardActions } from './components/current-proxy-card-actions'
import { CurrentProxyPathBanner } from './components/current-proxy-path-banner'
import { CurrentProxySelectors } from './components/current-proxy-selectors'
import { CurrentProxyStatusIcon } from './components/current-proxy-status-icon'
import { ProxyInfoDisplay } from './components/proxy-info-display'
import { useCurrentProxyCardController } from './hooks/use-current-proxy-card-controller'

export const CurrentProxyCard = () => {
  const {
    currentDelay,
    currentPathText,
    currentProxy,
    defaultLatencyTimeout,
    groups,
    isCoreDataPending,
    isGlobalMode,
    noActiveNodeLabel,
    onCheckAllDelay,
    onGroupSelectChange,
    onOpenProxies,
    onProxyChange,
    onSortTypeChange,
    pageTitle,
    proxiesLabel,
    proxyOptions,
    records,
    refreshDelayLabel,
    selectedGroup,
    selectedProxy,
    signalVisual,
    sortTooltip,
    sortType,
  } = useCurrentProxyCardController()

  return (
    <EnhancedCard
      title={pageTitle}
      icon={
        <CurrentProxyStatusIcon
          currentDelay={currentDelay}
          currentProxy={currentProxy}
          noActiveNodeLabel={noActiveNodeLabel}
          refreshDelayLabel={refreshDelayLabel}
          signalVisual={signalVisual}
          timeout={defaultLatencyTimeout}
        />
      }
      iconColor={currentProxy ? 'primary' : undefined}
      noContentPadding
      action={
        <CurrentProxyCardActions
          onCheckAllDelay={onCheckAllDelay}
          onOpenProxies={onOpenProxies}
          onSortTypeChange={onSortTypeChange}
          proxiesLabel={proxiesLabel}
          refreshDelayLabel={refreshDelayLabel}
          sortTooltip={sortTooltip}
          sortType={sortType}
        />
      }
    >
      {isCoreDataPending ? (
        <div className="py-2" />
      ) : (
        <div className="px-3 pt-1.5 pb-3">
          <CurrentProxyPathBanner pathText={currentPathText} />

          <div className="flex items-center gap-4">
            <div className="h-9 min-w-0 flex-1">
              <ProxyInfoDisplay
                proxy={currentProxy}
                delay={currentDelay}
                isGlobalMode={isGlobalMode}
                timeout={defaultLatencyTimeout}
              />
            </div>

            {currentProxy && (
              <CurrentProxySelectors
                defaultLatencyTimeout={defaultLatencyTimeout}
                groups={groups}
                isGlobalMode={isGlobalMode}
                onGroupChange={onGroupSelectChange}
                onProxyChange={onProxyChange}
                proxyOptions={proxyOptions}
                records={records}
                selectedGroup={selectedGroup}
                selectedProxy={selectedProxy}
              />
            )}
          </div>
        </div>
      )}
    </EnhancedCard>
  )
}
