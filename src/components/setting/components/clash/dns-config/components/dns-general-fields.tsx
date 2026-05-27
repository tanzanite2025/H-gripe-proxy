/**
 * DNS 通用字段组件
 * 包含基础配置字段
 */

import { TextField, Select, Box } from '@/components/tailwind'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'

import type { DnsFormValues } from '../utils/dns-helpers'

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
      <h2 className="uds-card-title text-base font-bold mt-4 mb-4">
        {t('settings.modals.dns.sections.general')}
      </h2>

      <Box className="flex items-center justify-between py-2 px-1">
        <div className="text-sm font-medium">
          {t('settings.modals.dns.fields.enable')}
        </div>
        <Switch
          edge="end"
          checked={values.enable}
          onChange={onChange('enable')}
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div className="text-sm font-medium">
          {t('settings.modals.dns.fields.listen')}
        </div>
        <TextField
          size="small"
          autoComplete="off"
          spellCheck="false"
          value={values.listen}
          onChange={onChange('listen')}
          placeholder=":53"
          className="w-[150px]"
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div className="text-sm font-medium">
          {t('settings.modals.dns.fields.enhancedMode')}
        </div>
        <Select
          size="small"
          value={values.enhancedMode}
          onChange={onChange('enhancedMode')}
          className="w-[150px]"
        >
          <option value="fake-ip">fake-ip</option>
          <option value="redir-host">redir-host</option>
        </Select>
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div className="text-sm font-medium">
          {t('settings.modals.dns.fields.fakeIpRange')}
        </div>
        <TextField
          size="small"
          autoComplete="off"
          spellCheck="false"
          value={values.fakeIpRange}
          onChange={onChange('fakeIpRange')}
          placeholder="198.18.0.1/16"
          className="w-[150px]"
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div className="text-sm font-medium">
          {t('settings.modals.dns.fields.fakeIpFilterMode')}
        </div>
        <Select
          size="small"
          value={values.fakeIpFilterMode}
          onChange={onChange('fakeIpFilterMode')}
          className="w-[150px]"
        >
          <option value="blacklist">blacklist</option>
          <option value="whitelist">whitelist</option>
        </Select>
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div>
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.ipv6.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.ipv6.description')}
          </div>
        </div>
        <Switch
          edge="end"
          checked={values.ipv6}
          onChange={onChange('ipv6')}
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div>
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.preferH3.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.preferH3.description')}
          </div>
        </div>
        <Switch
          edge="end"
          checked={values.preferH3}
          onChange={onChange('preferH3')}
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div>
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.respectRules.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.respectRules.description')}
          </div>
        </div>
        <Switch
          edge="end"
          checked={values.respectRules}
          onChange={onChange('respectRules')}
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div>
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.useHosts.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.useHosts.description')}
          </div>
        </div>
        <Switch
          edge="end"
          checked={values.useHosts}
          onChange={onChange('useHosts')}
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div>
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.useSystemHosts.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.useSystemHosts.description')}
          </div>
        </div>
        <Switch
          edge="end"
          checked={values.useSystemHosts}
          onChange={onChange('useSystemHosts')}
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div>
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.directPolicy.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.directPolicy.description')}
          </div>
        </div>
        <Switch
          edge="end"
          checked={values.directNameserverFollowPolicy}
          onChange={onChange('directNameserverFollowPolicy')}
        />
      </Box>
    </>
  )
}
