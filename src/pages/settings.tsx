import { GitHub, HelpOutlineRounded, Telegram } from '@mui/icons-material'
import { Box, ButtonGroup, IconButton, Grid } from '@mui/material'
import { useLockFn } from 'ahooks'
import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import SettingClash from '@/components/setting/setting-clash'
import { DnsStatsCard } from '@/components/setting/dns-stats-card'
import { DnsRoutingCard } from '@/components/setting/dns-routing-card'
import { TorConfigCard } from '@/components/setting/tor-config-card'
import { DnsLeakProtectionCard } from '@/components/setting/dns-leak-protection-card'
import SettingSystem from '@/components/setting/setting-system'
import SettingVergeAdvanced from '@/components/setting/setting-verge-advanced'
import SettingVergeBasic from '@/components/setting/setting-verge-basic'
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
          variant="contained"
          aria-label="Basic button group"
        >
          <IconButton
            size="medium"
            color="inherit"
            title={t('settings.page.actions.manual')}
            onClick={toGithubDoc}
          >
            <HelpOutlineRounded fontSize="inherit" />
          </IconButton>
          <IconButton
            size="medium"
            color="inherit"
            title={t('settings.page.actions.telegram')}
            onClick={toTelegramChannel}
          >
            <Telegram fontSize="inherit" />
          </IconButton>

          <IconButton
            size="medium"
            color="inherit"
            title={t('settings.page.actions.github')}
            onClick={toGithubRepo}
          >
            <GitHub fontSize="inherit" />
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
        <Grid size={{ xs: 6, sm: 6, md: 6, lg: 6 }} className="settings-page-grid__column">
          <Box className="uds-card-container settings-page-card">
            <SettingSystem onError={onError} />
          </Box>
          <Box className="uds-card-container settings-page-card">
            <SettingClash onError={onError} />
          </Box>
          <Box className="uds-card-container settings-page-card">
            <DnsStatsCard />
          </Box>
        </Grid>
        <Grid size={{ xs: 6, sm: 6, md: 6, lg: 6 }} className="settings-page-grid__column">
          <Box className="uds-card-container settings-page-card">
            <SettingVergeBasic onError={onError} />
          </Box>
          <Box className="uds-card-container settings-page-card">
            <DnsRoutingCard />
          </Box>
          <Box className="uds-card-container settings-page-card">
            <DnsLeakProtectionCard />
          </Box>
        </Grid>
        <Grid size={{ xs: 6, sm: 6, md: 12, lg: 6 }} className="settings-page-grid__column">
          <Box className="uds-card-container settings-page-card">
            <SettingVergeAdvanced onError={onError} />
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
