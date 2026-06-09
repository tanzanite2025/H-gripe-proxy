import { useTranslation } from 'react-i18next'

import { Alert, Snackbar } from '@/components/tailwind'

import { ScrollTopButton } from '../../../layout/scroll-top-button'
import { ProxyChainDrawer } from '../../proxy-chain-drawer'
import { StrategyPoolEditorDialog } from '../../strategy-pool-editor-dialog'
import type { ProxyGroupsController } from '../hooks/use-proxy-groups-controller'

import { ChainRuleHeader } from './chain-rule-header'
import { GroupSelectMenu } from './group-select-menu'
import { ProxyVirtualList } from './proxy-virtual-list'

interface ProxyGroupsChainViewProps {
  controller: ProxyGroupsController
}

export function ProxyGroupsChainView({
  controller,
}: ProxyGroupsChainViewProps) {
  const { t } = useTranslation()
  const proxyGroups = controller.proxiesData?.groups || []
  const showRuleHeader = controller.isRuleMode && proxyGroups.length > 0

  return (
    <>
      <div className="h-full">
        {showRuleHeader && (
          <ChainRuleHeader
            title={t('proxies.page.rules.title')}
            selectLabel={t('proxies.page.rules.select')}
            currentGroup={controller.currentGroup}
            canSelectGroup={controller.availableGroups.length > 0}
            onMenuOpen={controller.handleGroupMenuOpen}
          />
        )}

        <ProxyVirtualList
          {...controller.createProxyListProps(
            showRuleHeader ? 'calc(100% - 80px)' : 'calc(100% - 14px)',
          )}
        />
        <ScrollTopButton
          show={controller.showScrollTop}
          onClick={controller.scrollToTop}
        />
      </div>

      <Snackbar
        open={controller.duplicateWarning.open}
        autoHideDuration={3000}
        onClose={controller.handleCloseDuplicateWarning}
        anchorOrigin={{ vertical: 'top', horizontal: 'center' }}
      >
        <Alert
          onClose={controller.handleCloseDuplicateWarning}
          severity="warning"
          variant="filled"
        >
          {controller.duplicateWarning.message}
        </Alert>
      </Snackbar>

      <GroupSelectMenu
        anchorEl={controller.ruleMenuAnchor}
        groups={controller.availableGroups}
        selectedGroup={controller.activeSelectedGroup}
        emptyText="暂无可用代理组"
        onClose={controller.handleGroupMenuClose}
        onSelect={controller.handleGroupSelect}
      />

      <ProxyChainDrawer
        open={controller.isChainMode}
        proxyChain={controller.proxyChain}
        onUpdateChain={controller.setProxyChain}
        chainConfigData={controller.chainConfigData}
        mode={controller.displayMode}
        selectedGroup={controller.activeSelectedGroup}
        onClose={controller.handleCloseChainMode}
      />

      <StrategyPoolEditorDialog
        open={Boolean(controller.editingStrategyGroup)}
        group={controller.editingStrategyGroup}
        onClose={controller.closeStrategyGroupEditor}
        onSaved={controller.onProxies}
      />
    </>
  )
}
