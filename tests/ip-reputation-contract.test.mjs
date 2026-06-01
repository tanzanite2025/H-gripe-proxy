import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const servicePath = join(repoRoot, 'src', 'services', 'ip-reputation.ts')

test('ip reputation service normalizes tauri snake_case contract at the boundary', () => {
  const service = readFileSync(servicePath, 'utf8')

  assert.match(service, /export function normalizeIpReputation/)
  assert.match(service, /export function normalizeIpReputationConfig/)
  assert.match(service, /function normalizeRiskRoutingRule/)
  assert.match(service, /function serializeIpReputationConfig/)
  assert.match(service, /function serializeRiskRoutingRule/)

  for (const field of [
    'ip_type',
    'asn_org',
    'fraud_score',
    'risk_level',
    'is_proxy',
    'is_vpn',
    'is_tor',
    'country_code',
    'checked_at',
  ]) {
    assert.match(service, new RegExp(field), `${field} should be mapped`)
  }

  for (const field of [
    'cache_ttl',
    'routing_rules',
    'use_local_db',
    'domain_patterns',
    'required_ip_type',
    'max_fraud_score',
    'fallback_policy',
  ]) {
    assert.match(service, new RegExp(field), `${field} should be mapped`)
  }

  assert.match(service, /const IP_TYPE_ALIASES/)
  assert.match(service, /datacenter:\s*'Datacenter'/)
  assert.match(service, /residential:\s*'Residential'/)
  assert.match(service, /mobile:\s*'Mobile'/)
  assert.match(service, /education:\s*'Education'/)
  assert.match(service, /toTauriIpType/)
  assert.match(service, /required_ip_type: toTauriIpType/)

  assert.match(
    service,
    /ipReputationCheckIp[\s\S]*normalizeIpReputation/,
    'check_ip response should be normalized before UI reads it',
  )
  assert.match(
    service,
    /ipReputationGetCacheEntries[\s\S]*\.map\(normalizeIpReputation\)/,
    'cache entries should be normalized before UI reads them',
  )
})
