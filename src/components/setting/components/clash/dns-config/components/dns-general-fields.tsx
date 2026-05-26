/**
 * DNS 通用字段组件
 * 包含基础配置字段
 */

import {
  FormControl,
  ListItem,
  ListItemText,
  MenuItem,
  Select,
  styled,
  TextField,
  Typography,
} from '@mui/material'
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

interface DnsGeneralFieldsProps {
  values: DnsFormValues
  onChange: (field: string) => (event: any) => void
}

export const DnsGeneralFields = ({
  values,
  onChange,
}: DnsGeneralFieldsProps) => {
  const { t } = useTranslation()

  return (
    <>
      <Typography
        className="uds-card-title"
        variant="subtitle1"
        sx={{ mt: 1, mb: 1, fontWeight: 'bold' }}
      >
        {t('settings.modals.dns.sections.general')}
      </Typography>

      <Item>
        <ListItemText primary={t('settings.modals.dns.fields.enable')} />
        <Switch
          edge="end"
          checked={values.enable}
          onChange={onChange('enable')}
        />
      </Item>

      <Item>
        <ListItemText primary={t('settings.modals.dns.fields.listen')} />
        <TextField
          size="small"
          autoComplete="off"
          spellCheck="false"
          value={values.listen}
          onChange={onChange('listen')}
          placeholder=":53"
          sx={{ width: 150 }}
        />
      </Item>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.enhancedMode')}
        />
        <FormControl size="small" sx={{ width: 150 }}>
          <Select
            value={values.enhancedMode}
            onChange={onChange('enhancedMode')}
          >
            <MenuItem value="fake-ip">fake-ip</MenuItem>
            <MenuItem value="redir-host">redir-host</MenuItem>
          </Select>
        </FormControl>
      </Item>

      <Item>
        <ListItemText primary={t('settings.modals.dns.fields.fakeIpRange')} />
        <TextField
          size="small"
          autoComplete="off"
          spellCheck="false"
          value={values.fakeIpRange}
          onChange={onChange('fakeIpRange')}
          placeholder="198.18.0.1/16"
          sx={{ width: 150 }}
        />
      </Item>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.fakeIpFilterMode')}
        />
        <FormControl size="small" sx={{ width: 150 }}>
          <Select
            value={values.fakeIpFilterMode}
            onChange={onChange('fakeIpFilterMode')}
          >
            <MenuItem value="blacklist">blacklist</MenuItem>
            <MenuItem value="whitelist">whitelist</MenuItem>
          </Select>
        </FormControl>
      </Item>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.ipv6.label')}
          secondary={t('settings.modals.dns.fields.ipv6.description')}
        />
        <Switch
          edge="end"
          checked={values.ipv6}
          onChange={onChange('ipv6')}
        />
      </Item>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.preferH3.label')}
          secondary={t('settings.modals.dns.fields.preferH3.description')}
        />
        <Switch
          edge="end"
          checked={values.preferH3}
          onChange={onChange('preferH3')}
        />
      </Item>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.respectRules.label')}
          secondary={t('settings.modals.dns.fields.respectRules.description')}
        />
        <Switch
          edge="end"
          checked={values.respectRules}
          onChange={onChange('respectRules')}
        />
      </Item>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.useHosts.label')}
          secondary={t('settings.modals.dns.fields.useHosts.description')}
        />
        <Switch
          edge="end"
          checked={values.useHosts}
          onChange={onChange('useHosts')}
        />
      </Item>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.useSystemHosts.label')}
          secondary={t(
            'settings.modals.dns.fields.useSystemHosts.description',
          )}
        />
        <Switch
          edge="end"
          checked={values.useSystemHosts}
          onChange={onChange('useSystemHosts')}
        />
      </Item>

      <Item>
        <ListItemText
          primary={t('settings.modals.dns.fields.directPolicy.label')}
          secondary={t('settings.modals.dns.fields.directPolicy.description')}
        />
        <Switch
          edge="end"
          checked={values.directNameserverFollowPolicy}
          onChange={onChange('directNameserverFollowPolicy')}
        />
      </Item>
    </>
  )
}
