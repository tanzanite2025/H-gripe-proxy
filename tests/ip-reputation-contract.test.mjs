import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const servicePath = join(repoRoot, 'src', 'services', 'ip-reputation.ts')
const coordinatorPath = join(repoRoot, 'src', 'services', 'coordinator.ts')

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

test('advanced config service preserves ip reputation camel/snake contract on read and write', () => {
  const service = readFileSync(servicePath, 'utf8')
  const coordinator = readFileSync(coordinatorPath, 'utf8')

  assert.match(
    service,
    /export function serializeIpReputationConfig/,
    'ip reputation config serializer should be reusable by aggregate AdvancedConfig saves',
  )
  assert.match(coordinator, /normalizeIpReputationConfig/)
  assert.match(coordinator, /serializeIpReputationConfig/)
  assert.match(
    coordinator,
    /function normalizeAdvancedConfig[\s\S]*normalizeIpReputationConfig/,
    'aggregate AdvancedConfig normalization should normalize ip_reputation',
  )
  assert.match(
    coordinator,
    /function serializeAdvancedConfig[\s\S]*serializeIpReputationConfig/,
    'aggregate AdvancedConfig serialization should serialize ip_reputation',
  )
  assert.match(
    coordinator,
    /getAdvancedConfig[\s\S]*normalizeAdvancedConfig/,
    'aggregate AdvancedConfig loads should normalize ip_reputation before panels read it',
  )
  assert.match(
    coordinator,
    /saveAdvancedConfig[\s\S]*serializeAdvancedConfig/,
    'aggregate AdvancedConfig saves should serialize ip_reputation before Tauri receives it',
  )
  assert.match(
    coordinator,
    /validateAdvancedConfig[\s\S]*serializeAdvancedConfig/,
    'aggregate AdvancedConfig validation should use the same backend contract as saves',
  )
})
