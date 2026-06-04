import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const ruleConflict = readFileSync(
  new URL('../src/utils/rule-conflict.ts', import.meta.url),
  'utf8',
)
const profileRulesPanel = readFileSync(
  new URL('../src/components/profile/profile-rules-panel.tsx', import.meta.url),
  'utf8',
)
const ruleItem = readFileSync(
  new URL('../src/components/rule/rule-item.tsx', import.meta.url),
  'utf8',
)

test('profile rules panel never hides unresolved conflicts locally', () => {
  assert.doesNotMatch(profileRulesPanel, /RULE_CONFLICT_DISMISSAL_STORAGE_KEY/)
  assert.doesNotMatch(profileRulesPanel, /dismissedConflictRuleKeys/)
  assert.doesNotMatch(profileRulesPanel, /visibleConflicts/)
  assert.doesNotMatch(profileRulesPanel, /localStorage/)
  assert.match(profileRulesPanel, /getConflictSummary\(conflicts\)/)
  assert.match(profileRulesPanel, /conflicts\.length > 0/)
})

test('rule item only changes real runtime rule state, not conflict visibility', () => {
  assert.match(ruleItem, /disableRules\(\{ \[value\.index\]: !isDisabled \}\)/)
  assert.match(ruleItem, /deleteRule\(value\.index\)/)
  assert.doesNotMatch(ruleItem, /onConflictRuleResolved/)
})

test('rule conflict detector only suppresses rules with real disabled or deleted state', () => {
  assert.match(
    ruleConflict,
    /const activeRules = rules\.filter\(\s*\(r\) => !\(r\.extra\?\.disabled \|\| r\.extra\?\.deleted\),\s*\)/,
  )
  assert.doesNotMatch(ruleConflict, /getRuleConflictRuleKey/)
})
