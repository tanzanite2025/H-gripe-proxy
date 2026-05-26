/**
 * DNS 域名服务器字段组件
 * 包含各类域名服务器配置
 */

import { ListItem, ListItemText, styled, TextField } from '@mui/material'
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

interface DnsNameserverFieldsProps {
  values: DnsFormValues
  onChange: (field: string) => (event: any) => void
}

export const DnsNameserverFields = ({
  values,
  onChange,
}: DnsNameserverFieldsProps) => {
  const { t } = useTranslation()

  return (
    <>
      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.defaultNameserver.label')}
          secondary={t(
            'settings.modals.dns.fields.defaultNameserver.description',
          )}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={3}
          size="small"
          spellCheck="false"
          value={values.defaultNameserver}
          onChange={onChange('defaultNameserver')}
          placeholder="system,223.6.6.6, 8.8.8.8, 2400:3200::1, 2001:4860:4860::8888"
        />
      </Item>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.nameserver.label')}
          secondary={t('settings.modals.dns.fields.nameserver.description')}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={4}
          size="small"
          spellCheck="false"
          value={values.nameserver}
          onChange={onChange('nameserver')}
          placeholder="8.8.8.8, https://doh.pub/dns-query, https://dns.alidns.com/dns-query"
        />
      </Item>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.fallback.label')}
          secondary={t('settings.modals.dns.fields.fallback.description')}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={4}
          size="small"
          spellCheck="false"
          value={values.fallback}
          onChange={onChange('fallback')}
          placeholder="https://dns.alidns.com/dns-query, https://dns.google/dns-query, https://cloudflare-dns.com/dns-query"
        />
      </Item>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.proxy.label')}
          secondary={t('settings.modals.dns.fields.proxy.description')}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={3}
          size="small"
          spellCheck="false"
          value={values.proxyServerNameserver}
          onChange={onChange('proxyServerNameserver')}
          placeholder="https://doh.pub/dns-query, https://dns.alidns.com/dns-query"
        />
      </Item>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.directNameserver.label')}
          secondary={t(
            'settings.modals.dns.fields.directNameserver.description',
          )}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={3}
          size="small"
          spellCheck="false"
          value={values.directNameserver}
          onChange={onChange('directNameserver')}
          placeholder="system, 223.6.6.6"
        />
      </Item>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.fakeIpFilter.label')}
          secondary={t('settings.modals.dns.fields.fakeIpFilter.description')}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={4}
          size="small"
          spellCheck="false"
          value={values.fakeIpFilter}
          onChange={onChange('fakeIpFilter')}
          placeholder="*.lan, *.local, localhost.ptlogin2.qq.com"
        />
      </Item>

      <Item sx={{ flexDirection: 'column', alignItems: 'flex-start' }}>
        <ListItemText
          primary={t('settings.modals.dns.fields.nameserverPolicy.label')}
          secondary={t(
            'settings.modals.dns.fields.nameserverPolicy.description',
          )}
        />
        <TextField
          fullWidth
          multiline
          minRows={2}
          maxRows={4}
          size="small"
          spellCheck="false"
          value={values.nameserverPolicy}
          onChange={onChange('nameserverPolicy')}
          placeholder="+.arpa=10.0.0.1, rule-set:cn=https://doh.pub/dns-query;https://dns.alidns.com/dns-query"
        />
      </Item>
    </>
  )
}
