/**
 * DNS 域名服务器字段组件
 * 包含各类域名服务器配置
 */

import { TextField, Box } from '@/components/tailwind'
import { useTranslation } from 'react-i18next'

import type { DnsFormValues } from '../utils/dns-helpers'

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
      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.defaultNameserver.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.defaultNameserver.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.nameserver.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.nameserver.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.fallback.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.fallback.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.proxy.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.proxy.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.directNameserver.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.directNameserver.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.fakeIpFilter.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.fakeIpFilter.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>

      <Box className="flex flex-col items-start py-2 px-1">
        <div className="mb-2">
          <div className="text-sm font-medium">
            {t('settings.modals.dns.fields.nameserverPolicy.label')}
          </div>
          <div className="text-xs text-text-secondary">
            {t('settings.modals.dns.fields.nameserverPolicy.description')}
          </div>
        </div>
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
          className="[&_textarea]:leading-[1.5] [&_textarea]:text-sm [&_textarea]:resize-y"
        />
      </Box>
    </>
  )
}
