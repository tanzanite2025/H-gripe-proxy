import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import SettingClash from '@/components/setting/setting-clash'
import SettingSystem from '@/components/setting/setting-system'
import SettingVergeBasic from '@/components/setting/setting-verge-basic'
import { Box, Grid } from '@/components/tailwind'
import { showNotice } from '@/services/notice-service'

const SettingPage = () => {
  const { t } = useTranslation()

  const onError = (err: any) => {
    showNotice.error(err)
  }

  return (
    <BasePage
      title={t('settings.page.title')}
    >
      <Grid
        container
        spacing={3}
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
        </Grid>
        <Grid item xs={6} sm={6} md={6} lg={6} className="settings-page-grid__column">
          <Box className="uds-card-container settings-page-card">
            <SettingVergeBasic onError={onError} />
          </Box>
        </Grid>
      </Grid>
    </BasePage>
  )
}

export default SettingPage
