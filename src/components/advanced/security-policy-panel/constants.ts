export const RULE_TYPES = [
  'DOMAIN',
  'DOMAIN-SUFFIX',
  'DOMAIN-KEYWORD',
  'IP-CIDR',
  'SRC-IP-CIDR',
  'GEOIP',
  'PROCESS-NAME',
  'PROCESS-PATH',
  'IN-TYPE',
  'IN-USER',
  'IN-NAME',
  'NETWORK',
  'UID',
  'AND',
  'OR',
  'NOT',
  'SUB-RULE',
] as const

export const LOGICAL_RULE_TYPES = new Set(['AND', 'OR', 'NOT'])

export function createEmptyPolicy(): ISecurityPolicy {
  return {
    name: '',
    enabled: true,
    description: '',
    rules: [],
  }
}

export function createEmptyRule(): IPolicyRule {
  return {
    ruleType: 'DOMAIN',
    payload: '',
    proxy: 'REJECT',
  }
}
