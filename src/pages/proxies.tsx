import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import { ProxyDelaySettings } from '@/components/proxy/proxy-delay-settings'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { Box, Grid } from '@/components/tailwind'

import {
  DnsLeakDialog,
  ProxyDetectionDialog,
} from './proxies-page/proxy-page-detection-dialogs'
import { ProxyPageStrategyPools } from './proxies-page/proxy-page-strategy-pools'
import { ProxyPageTitle } from './proxies-page/proxy-page-title'
import { ProxyPageToolbar } from './proxies-page/proxy-page-toolbar'
import { useProxiesPageController } from './proxies-page/use-proxies-page-controller'

const ProxyPage = () => {
  const { t } = useTranslation()
  const { isChainMode, chainConfigData, onToggleChainMode } =
    useProxiesPageController()
  const [proxyDetectionOpen, setProxyDetectionOpen] = useState(false)
  const [dnsLeakOpen, setDnsLeakOpen] = useState(false)

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
        <Grid item xs={12} className="h-full overflow-hidden">
          <Box className="flex h-full min-h-0 flex-col overflow-hidden">
            <ProxyPageToolbar
              isChainMode={isChainMode}
              toggleLabel={t('proxies.page.actions.toggleChain')}
              proxyDetectionLabel={t('proxies.page.actions.proxyDetection')}
              dnsLeakLabel={t('proxies.page.actions.dnsLeak')}
              onToggleChainMode={onToggleChainMode}
              onOpenProxyDetection={() => setProxyDetectionOpen(true)}
              onOpenDnsLeak={() => setDnsLeakOpen(true)}
            />

            <ProxyDelaySettings />

            <ProxyPageStrategyPools />

            <Box className="min-h-0 flex-1 overflow-hidden">
              <ProxyGroups
                isChainMode={isChainMode}
                chainConfigData={chainConfigData}
                onCloseChainMode={onToggleChainMode}
              />
            </Box>
          </Box>
        </Grid>
      </Grid>

      <ProxyDetectionDialog
        open={proxyDetectionOpen}
        onClose={() => setProxyDetectionOpen(false)}
      />
      <DnsLeakDialog open={dnsLeakOpen} onClose={() => setDnsLeakOpen(false)} />
    </BasePage>
  )
}

export default ProxyPage
