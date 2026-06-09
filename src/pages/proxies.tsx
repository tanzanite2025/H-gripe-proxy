import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import { ProxyDelaySettings } from '@/components/proxy/proxy-delay-settings'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { Box, Grid } from '@/components/tailwind'

import { ProxyPageModeCard } from './proxies-page/proxy-page-mode-card'
import { ProxyPageSideCards } from './proxies-page/proxy-page-side-cards'
import { ProxyPageTitle } from './proxies-page/proxy-page-title'
import { ProxyPageToolbar } from './proxies-page/proxy-page-toolbar'
import { useProxiesPageController } from './proxies-page/use-proxies-page-controller'

const ProxyPage = () => {
  const { t } = useTranslation()
  const {
    isChainMode,
    chainConfigData,
    proxyDisplayMode,
    onChangeMode,
    onToggleChainMode,
  } = useProxiesPageController()

  return (
    <BasePage
      full
      contentClassName="h-full pt-[15px]"
      title={
        isChainMode ? (
          <ProxyPageTitle
            title={t('proxies.page.title.chainMode')}
            warning={t('proxies.page.chain.warning')}
          />
        ) : (
          t('proxies.page.title.default')
        )
      }
    >
      <Grid container spacing={3} columns={12} className="h-full">
        <Grid item xs={12} lg={6} xl={6} className="h-full overflow-hidden">
          <Box className="flex h-full min-h-0 flex-col overflow-hidden">
            <ProxyPageToolbar
              isChainMode={isChainMode}
              toggleLabel={t('proxies.page.actions.toggleChain')}
              onToggleChainMode={onToggleChainMode}
            />

            <ProxyDelaySettings />

            <ProxyPageModeCard
              mode={proxyDisplayMode}
              onChangeMode={onChangeMode}
            />

            <Box className="min-h-0 flex-1 overflow-hidden">
              <ProxyGroups
                mode={proxyDisplayMode}
                isChainMode={isChainMode}
                chainConfigData={chainConfigData}
                onCloseChainMode={onToggleChainMode}
              />
            </Box>
          </Box>
        </Grid>

        <Grid item xs={12} lg={6} xl={6} className="h-full overflow-hidden">
          <ProxyPageSideCards />
        </Grid>
      </Grid>
    </BasePage>
  )
}

export default ProxyPage
