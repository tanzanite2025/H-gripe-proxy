import type { IpReputation } from './model'

const riskColorMap: Record<IpReputation['riskLevel'], string> = {
  Low: 'text-green-600',
  Medium: 'text-yellow-600',
  High: 'text-orange-600',
  VeryHigh: 'text-red-600',
}

export function getIpTypeText(ipType: string): string {
  switch (ipType) {
    case 'Datacenter':
      return '机房 IP'
    case 'Residential':
      return '住宅特征'
    case 'Mobile':
      return '移动特征'
    case 'Education':
      return '教育网特征'
    default:
      return '未知'
  }
}

export function getResidentialStateText(state: string): string {
  switch (state) {
    case 'notResidential':
      return '非住宅'
    case 'observedResidential':
      return '观测似住宅'
    case 'verifiedResidential':
      return '已验证住宅'
    default:
      return '未确认'
  }
}

export function getRiskLevelText(riskLevel: string): string {
  switch (riskLevel) {
    case 'Low':
      return '低风险'
    case 'Medium':
      return '中风险'
    case 'High':
      return '高风险'
    case 'VeryHigh':
      return '极高风险'
    default:
      return '未知'
  }
}

export function getRiskLevelColor(riskLevel: string): string {
  return riskColorMap[riskLevel as IpReputation['riskLevel']] ?? 'text-gray-600'
}
