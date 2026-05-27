/**
 * DNS 回退过滤字段组件
 * 包含 fallback-filter 相关配置
 */

import { TextField, Box } from '@/components/tailwind'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'

import type { DnsFormValues } from '../utils/dns-helpers'

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
      <h3 className="text-sm font-bold mt-8 mb-4">
        {t('settings.modals.dns.sections.fallbackFilter')}
      </h3>

      <Box className="flex items-center justify-between py-2 px-1">
        <div>
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.geoipFiltering.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.geoipFiltering.description')}
          </div>
        </div>
        <Switch
          edge="end"
          checked={values.fallbackGeoip}
          onChange={onChange('fallbackGeoip')}
        />
      </Box>

      <Box className="flex items-center justify-between py-2 px-1">
        <div className="text-sm font-medium">
          {t('settings.modals.dns.fields.geoipCode')}
        </div>
        <TextField
          size="small"
          autoComplete="off"
          spellCheck="false"
          value={values.fallbackGeoipCode}
          onChange={onChange('fallbackGeoipCode')}
          placeholder="CN"
          className="w-[100px]"
        />
      </Box>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.fallbackIpCidr.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.fallbackIpCidr.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.fallbackDomain.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.fallbackDomain.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>
    </>
  )
}
