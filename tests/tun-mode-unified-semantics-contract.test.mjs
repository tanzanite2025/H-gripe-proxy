import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const tunEnhancePath = join(repoRoot, 'src-tauri', 'src', 'enhance', 'tun.rs')
const enhanceModPath = join(repoRoot, 'src-tauri', 'src', 'enhance', 'mod.rs')
const securityPolicyPath = join(repoRoot, 'src-tauri', 'src', 'core', 'security_policy.rs')
const securityPolicyPanelPath = join(repoRoot, 'src', 'components', 'advanced', 'security-policy-panel.tsx')

test('TUN remains an ingress toggle and does not install a separate TUN rule branch', () => {
  const tunEnhance = readFileSync(tunEnhancePath, 'utf8')
  const enhanceMod = readFileSync(enhanceModPath, 'utf8')

  assert.doesNotMatch(tunEnhance, /apply_tun_security_policy/)
  assert.doesNotMatch(tunEnhance, /tun-security/)
  assert.doesNotMatch(tunEnhance, /revise!\(tun_map,\s*"rule"/)
  assert.doesNotMatch(tunEnhance, /"sub-rules"/)
  assert.doesNotMatch(enhanceMod, /apply_tun_security_policy/)
})

test('security policies use the same runtime rule path regardless of legacy tunOnly', () => {
  const securityPolicy = readFileSync(securityPolicyPath, 'utf8')

  assert.match(securityPolicy, /pub tun_only: bool/)
  assert.match(securityPolicy, /compatibility/)
  assert.doesNotMatch(securityPolicy, /TUN_SECURITY_SUB_RULE/)
  assert.doesNotMatch(securityPolicy, /delete_sub_rule_by_source/)
  assert.doesNotMatch(securityPolicy, /if policy\.tun_only/)
  assert.match(securityPolicy, /create_rule\([\s\S]*?None,\s*None,[\s\S]*?\)/)
})

test('security policy UI no longer exposes a TUN-only branch control', () => {
  const panel = readFileSync(securityPolicyPanelPath, 'utf8')

  assert.doesNotMatch(panel, /tunOnly/)
  assert.doesNotMatch(panel, /TUN/)
})
