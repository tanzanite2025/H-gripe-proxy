import { LOGICAL_RULE_TYPES } from './constants'

export function clonePolicy(policy: ISecurityPolicy): ISecurityPolicy {
  return {
    ...policy,
    rules: policy.rules.map((rule) => ({ ...rule })),
  }
}

export function findAppliedPolicyState(
  states: IAppliedPolicyState[],
  name: string,
) {
  return states.find((state) => state.name === name) ?? null
}

export function getRulePayloadHelperText(ruleType: string) {
  if (LOGICAL_RULE_TYPES.has(ruleType)) {
    return '逻辑规则 payload 示例：((DOMAIN,example.com),(DOMAIN-SUFFIX,example.org))'
  }

  if (ruleType === 'SUB-RULE') {
    return '填写要引用的子规则名称。'
  }

  return undefined
}

export function validatePolicy(
  policy: ISecurityPolicy,
  policies: ISecurityPolicy[],
  editingIndex: number,
) {
  if (!policy.name.trim()) {
    return '策略名称不能为空。'
  }

  if (policy.rules.length === 0) {
    return '至少需要添加一条规则。'
  }

  const duplicated = policies.some(
    (item, index) => index !== editingIndex && item.name === policy.name,
  )
  if (duplicated) {
    return `策略“${policy.name}”已存在。`
  }

  for (const [index, rule] of policy.rules.entries()) {
    if (!rule.payload.trim()) {
      return `第 ${index + 1} 条规则的 payload 不能为空。`
    }

    if (!rule.proxy.trim()) {
      return `第 ${index + 1} 条规则的目标代理不能为空。`
    }
  }

  return null
}
