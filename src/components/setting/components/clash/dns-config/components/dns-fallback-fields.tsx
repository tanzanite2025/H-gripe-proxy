/**
 * DNS 回退过滤字段组件
 * 包含 fallback-filter 相关配置
 */

import { ListItem, ListItemText, styled, TextField, Typography } from '@mui/material'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'

import type { DnsFormValues } from '../utils/dns-helpers'

const Item = styled(ListItem)(() => ({
  padding: '5px 2px',
  '& textarea': {
    lineHeight: 1.5,
    fontSize: 14,
    resize: 'vertical',
  },
}))

interface DnsFallbackFieldsProps {
  values: DnsFormValues
  onChange: (field: string) => (event: any) => void
}

export const DnsFallbackFields = ({
  values,
  onChange,
}: DnsFallbackFieldsProps) => {
  const { t } = useTranslation()

  return (
    <>
      <Typography
        variant="subtitle2"
        sx={{ mt: 2, mb: 1, fontWeight: 'bold' }}
      >
        {t('settings.modals.dns.sections.fallbackFilter')}
      </Typography>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.geoipFiltering.label')}
          secondary={t(
            'settings.modals.dns.fields.geoipFiltering.description',
          )}
        />
        <Switch
          edge="end"
          checked={values.fallbackGeoip}
          onChange={onChange('fallbackGeoip')}
        />
      </Item>

      <Item>
        <ListItemText primary={t('settings.modals.dns.fields.geoipCode')} />
        <TextField
          size="small"
          autoComplete="off"
          spellCheck="false"
          value={values.fallbackGeoipCode}
          onChange={onChange('fallbackGeoipCode')}
          placeholder="CN"
          sx={{ width: 100 }}
        />
      </Item>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.fallbackIpCidr.label')}
          secondary={t(
            'settings.modals.dns.fields.fallbackIpCidr.description',
          )}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={3}
          size="small"
          spellCheck="false"
          value={values.fallbackIpcidr}
          onChange={onChange('fallbackIpcidr')}
          placeholder="240.0.0.0/4, 127.0.0.1/8"
        />
      </Item>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.fallbackDomain.label')}
          secondary={t(
            'settings.modals.dns.fields.fallbackDomain.description',
          )}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={3}
          size="small"
          spellCheck="false"
          value={values.fallbackDomain}
          onChange={onChange('fallbackDomain')}
          placeholder="+.google.com, +.facebook.com, +.youtube.com"
        />
      </Item>
    </>
  )
}
