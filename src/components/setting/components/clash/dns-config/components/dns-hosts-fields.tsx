/**
 * DNS Hosts 字段组件
 * 包含 Hosts 映射配置
 */

import { TextField, Box } from '@/components/tailwind'
import { useTranslation } from 'react-i18next'

import type { DnsFormValues } from '../utils/dns-helpers'

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
      <h2 className="uds-card-title text-base font-bold mt-12 mb-0">
        {t('settings.modals.dns.sections.hosts')}
      </h2>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.hosts.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.hosts.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>
    </>
  )
}
