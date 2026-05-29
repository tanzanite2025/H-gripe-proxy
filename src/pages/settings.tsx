import { useLockFn } from 'ahooks'
import { HelpCircle, Send } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import SettingClash from '@/components/setting/setting-clash'
import SettingDns from '@/components/setting/setting-dns'
import SettingSystem from '@/components/setting/setting-system'
import SettingVergeTools from '@/components/setting/setting-verge-advanced'
import SettingVergeBasic from '@/components/setting/setting-verge-basic'
import { TorConfigCard } from '@/components/setting/tor-config-card'
import { Box, ButtonGroup, IconButton, Grid } from '@/components/tailwind'
import { openWebUrl } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

const SettingPage = () => {
  const { t } = useTranslation()

  const onError = (err: any) => {
    showNotice.error(err)
  }

  const toGithubRepo = useLockFn(() => {
    return openWebUrl('https://github.com/tanzanite2025/clash-verge-optimized')
  })

  const toGithubDoc = useLockFn(() => {
    return openWebUrl('https://github.com/tanzanite2025/clash-verge-optimized#readme')
  })

  const toTelegramChannel = useLockFn(() => {
    return openWebUrl('https://t.me/clash_verge_re')
  })

  return (
    <BasePage
      title={t('settings.page.title')}
      header={
        <ButtonGroup
          className="uds-toolbar uds-toolbar--icon"
          variant="primary"
          aria-label="Basic button group"
        >
          <IconButton
            size="medium"
            color="inherit"
            title={t('settings.page.actions.manual')}
            onClick={toGithubDoc}
          >
            <HelpCircle className="h-5 w-5" />
          </IconButton>
          <IconButton
            size="medium"
            color="inherit"
            title={t('settings.page.actions.telegram')}
            onClick={toTelegramChannel}
          >
            <Send className="h-5 w-5" />
          </IconButton>

          <IconButton
            size="medium"
            color="inherit"
            title={t('settings.page.actions.github')}
            onClick={toGithubRepo}
          >
            <span className="text-xs font-black tracking-wide">GH</span>
          </IconButton>
        </ButtonGroup>
      }
    >
      <Grid
        container
        spacing={1.5}
        columns={{ xs: 6, sm: 6, md: 12, lg: 18 }}
        className="settings-page-grid"
      >
        <Grid item xs={6} sm={6} md={6} lg={6} className="settings-page-grid__column">
          <Box className="uds-card-container settings-page-card">
            <SettingSystem onError={onError} />
          </Box>
          <Box className="uds-card-container settings-page-card">
            <SettingClash onError={onError} />
          </Box>
          <Box className="uds-card-container settings-page-card">
            <SettingDns />
          </Box>
        </Grid>
        <Grid item xs={6} sm={6} md={6} lg={6} className="settings-page-grid__column">
          <Box className="uds-card-container settings-page-card">
            <SettingVergeBasic onError={onError} />
          </Box>
        </Grid>
        <Grid item xs={6} sm={6} md={12} lg={6} className="settings-page-grid__column">
          <Box className="uds-card-container settings-page-card">
            <SettingVergeTools onError={onError} />
          </Box>
          <Box className="uds-card-container settings-page-card">
            <TorConfigCard />
          </Box>
        </Grid>
      </Grid>
    </BasePage>
  )
}

export default SettingPage
