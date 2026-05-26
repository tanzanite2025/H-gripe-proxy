/**
 * DNS Hosts 字段组件
 * 包含 Hosts 映射配置
 */

import { ListItem, ListItemText, styled, TextField, Typography } from '@mui/material'
import { useTranslation } from 'react-i18next'

import type { DnsFormValues } from '../utils/dns-helpers'

const Item = styled(ListItem)(() => ({
  padding: '5px 2px',
  '& textarea': {
    lineHeight: 1.5,
    fontSize: 14,
    resize: 'vertical',
  },
}))

interface DnsHostsFieldsProps {
  values: DnsFormValues
  onChange: (field: string) => (event: any) => void
}

export const DnsHostsFields = ({
  values,
  onChange,
}: DnsHostsFieldsProps) => {
  const { t } = useTranslation()

  return (
    <>
      <Typography
        className="uds-card-title"
        variant="subtitle1"
        sx={{ mt: 3, mb: 0, fontWeight: 'bold' }}
      >
        {t('settings.modals.dns.sections.hosts')}
      </Typography>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.hosts.label')}
          secondary={t('settings.modals.dns.fields.hosts.description')}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={4}
          size="small"
          spellCheck="false"
          value={values.hosts}
          onChange={onChange('hosts')}
          placeholder="*.clash.dev=127.0.0.1, alpha.clash.dev=::1, test.com=1.1.1.1;2.2.2.2, baidu.com=google.com"
        />
      </Item>
    </>
  )
}
