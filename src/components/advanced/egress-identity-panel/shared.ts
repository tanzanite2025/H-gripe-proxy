import type {
  DnsMode,
  EgressFailoverPolicy,
  IpType,
} from '@/services/coordinator'

export interface EgressProfileOption {
  value: string
  label: string
}

export interface EgressIdentityPreviewFormState {
  process_name: string
  exe_path: string
  shortcut_id: string
  domain: string
  source_ip: string
  source_port: string
  available_nodes: string
}

export const emptyEgressIdentityPreviewForm: EgressIdentityPreviewFormState = {
  process_name: '',
  exe_path: '',
  shortcut_id: '',
  domain: '',
  source_ip: '',
  source_port: '',
  available_nodes: '',
}

export const dnsModeOptions: { value: DnsMode; label: string }[] = [
  { value: 'Inherit', label: '继承' },
  { value: 'Hijack', label: '强制劫持' },
  { value: 'Remote', label: '强制远端 DNS' },
]

export const failoverOptions: {
  value: EgressFailoverPolicy
  label: string
}[] = [
  { value: 'Block', label: '阻止' },
  { value: 'Manual', label: '手动确认' },
  { value: 'AutoSwitch', label: '自动切换' },
]

export const ipTypeOptions: { value: '' | IpType; label: string }[] = [
  { value: '', label: '不限制' },
  { value: 'Datacenter', label: '机房 IP' },
  { value: 'Residential', label: '住宅 IP' },
  { value: 'Mobile', label: '移动 IP' },
  { value: 'Unknown', label: '未知' },
]

export const splitList = (value: string) =>
  value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean)

export const joinList = (values: string[]) => values.join(', ')
