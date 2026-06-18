import type { Rule } from '@/types/mihomo'

export interface RuleConflict {
  /** The rule that is shadowed (will never match) */
  shadowed: Rule
  /** The rule that shadows it (matches first) */
  shadower: Rule
  /** Conflict type description */
  reason: string
  /** Severity: 'error' = always shadowed, 'warning' = likely shadowed */
  severity: 'error' | 'warning'
}

/**
 * Detect rule conflicts where a later rule is shadowed by an earlier rule
 * and will never be reached during matching.
 *
 * Detection strategy:
 * 1. DOMAIN vs DOMAIN: same payload, later one is shadowed
 * 2. DOMAIN vs DOMAIN-SUFFIX: domain "x.com" shadowed by suffix "x.com" or ".x.com"
 * 3. DOMAIN-SUFFIX vs DOMAIN-SUFFIX: ".x.y.com" shadowed by ".y.com"
 * 4. DOMAIN-KEYWORD vs DOMAIN/DOMAIN-SUFFIX: keyword match is broader, so
 *    a specific domain rule after a keyword rule that matches it is shadowed
 */
export function detectRuleConflicts(rules: Rule[]): RuleConflict[] {
  const conflicts: RuleConflict[] = []
  const activeRules = rules.filter(
    (r) => !(r.extra?.disabled || r.extra?.deleted),
  )

  // Index active rules by type for fast lookup
  const domainMap = new Map<string, Rule>() // exact domain -> first matching rule
  const suffixList: Array<{ suffix: string; rule: Rule }> = []
  const keywordList: Array<{ keyword: string; rule: Rule }> = []

  for (const rule of activeRules) {
    const type = rule.type
    const payload = rule.payload.toLowerCase()

    switch (type) {
      case 'Domain': {
        // Check if this exact domain was already matched by an earlier rule
        if (domainMap.has(payload)) {
          conflicts.push({
            shadowed: rule,
            shadower: domainMap.get(payload)!,
            reason: `域名 "${payload}" 已被前面的 Domain 规则匹配`,
            severity: 'error',
          })
        }
        // Check if this domain is matched by an earlier suffix rule
        for (const { suffix, rule: suffixRule } of suffixList) {
          if (domainEndsWith(payload, suffix)) {
            conflicts.push({
              shadowed: rule,
              shadower: suffixRule,
              reason: `域名 "${payload}" 已被前面的 DomainSuffix "${suffix}" 规则匹配`,
              severity: 'error',
            })
            break
          }
        }
        // Check if this domain is matched by an earlier keyword rule
        for (const { keyword, rule: kwRule } of keywordList) {
          if (payload.includes(keyword)) {
            conflicts.push({
              shadowed: rule,
              shadower: kwRule,
              reason: `域名 "${payload}" 已被前面的 DomainKeyword "${keyword}" 规则匹配`,
              severity: 'warning',
            })
            break
          }
        }
        domainMap.set(payload, rule)
        break
      }

      case 'DomainSuffix': {
        const suffix = payload.startsWith('.') ? payload : '.' + payload
        // Check if this suffix is shadowed by a broader suffix
        for (const { suffix: existingSuffix, rule: existingRule } of suffixList) {
          if (domainEndsWith(suffix.slice(1), existingSuffix)) {
            conflicts.push({
              shadowed: rule,
              shadower: existingRule,
              reason: `后缀 "${payload}" 是 "${existingRule.payload}" 的子集，已被匹配`,
              severity: 'error',
            })
            break
          }
        }
        suffixList.push({ suffix, rule })
        break
      }

      case 'DomainKeyword': {
        keywordList.push({ keyword: payload, rule })
        break
      }

      // GEOIP, IPCIDR, etc. — these are harder to detect statically
      // and are lower priority for conflict detection
      default:
        break
    }
  }

  return conflicts
}

/**
 * Check if a domain ends with a suffix pattern.
 * e.g. domainEndsWith("www.google.com", ".google.com") => true
 * e.g. domainEndsWith("google.com", ".com") => true
 * e.g. domainEndsWith("google.com", "google.com") => true
 */
function domainEndsWith(domain: string, suffix: string): boolean {
  if (suffix.startsWith('.')) {
    return domain.endsWith(suffix) || domain === suffix.slice(1)
  }
  return domain === suffix || domain.endsWith('.' + suffix)
}

/**
 * Get a summary of rule conflicts for display
 */
export function getConflictSummary(conflicts: RuleConflict[]): {
  errorCount: number
  warningCount: number
  shadowedIndices: Set<number>
} {
  let errorCount = 0
  let warningCount = 0
  const shadowedIndices = new Set<number>()

  for (const c of conflicts) {
    if (c.severity === 'error') errorCount++
    else warningCount++
    shadowedIndices.add(c.shadowed.index)
  }

  return { errorCount, warningCount, shadowedIndices }
}
