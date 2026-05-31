import { useLockFn } from 'ahooks'
import { useState } from 'react'
import { createRule } from 'tauri-plugin-mihomo-api'

import { BaseDialog } from '@/components/base'
import { Select, TextField } from '@/components/tailwind'
import { useAppRefreshers } from '@/providers/app-data-context'
import { showNotice } from '@/services/notice-service'

const RULE_TYPES = [
  'Domain',
  'DomainSuffix',
  'DomainKeyword',
  'DomainRegex',
  'GeoSite',
  'GeoIP',
  'IPCIDR',
  'SrcIPCIDR',
  'IPASN',
  'SrcIPASN',
  'DstPort',
  'SrcPort',
  'InPort',
  'InUser',
  'InName',
  'InType',
  'ProcessName',
  'ProcessPath',
  'Network',
  'RuleSet',
]

interface Props {
  open: boolean
  onClose: () => void
}

export const CreateRuleDialog = (props: Props) => {
  const { open, onClose } = props
  const { refreshRules } = useAppRefreshers()
  const [ruleType, setRuleType] = useState('DOMAIN')
  const [payload, setPayload] = useState('')
  const [proxy, setProxy] = useState('DIRECT')

  const handleCreate = useLockFn(async () => {
    if (!payload.trim()) {
      showNotice.error('Payload is required')
      return
    }
    try {
      await createRule(ruleType, payload.trim(), proxy.trim())
      await refreshRules()
      showNotice.success(`Rule created: ${ruleType},${payload.trim()},${proxy.trim()}`)
      setPayload('')
      onClose()
    } catch (err) {
      showNotice.error(`Failed to create rule: ${err}`)
    }
  })

  return (
    <BaseDialog
      open={open}
      title="Create Runtime Rule"
      okBtn="Create"
      cancelBtn="Cancel"
      onOk={handleCreate}
      onClose={onClose}
    >
      <div className="flex flex-col gap-4 py-2">
        <div>
          <label className="mb-1 block text-xs font-semibold uppercase tracking-widest text-text-secondary">
            Rule Type
          </label>
          <Select
            value={ruleType}
            onChange={(val: string | number) => setRuleType(String(val))}
            options={RULE_TYPES.map((type) => ({ value: type, label: type }))}
            size="small"
            fullWidth
          />
        </div>

        <TextField
          label="Payload"
          value={payload}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPayload(e.target.value)}
          placeholder="e.g. google.com"
          size="small"
          fullWidth
        />

        <TextField
          label="Proxy / Policy"
          value={proxy}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setProxy(e.target.value)}
          placeholder="e.g. DIRECT, REJECT, Proxy-Name"
          size="small"
          fullWidth
        />
      </div>
    </BaseDialog>
  )
}
