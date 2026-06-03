import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const coordinatorPath = join(repoRoot, 'src', 'services', 'coordinator.ts')

test('advanced config service exposes ingress countermeasure config contract', () => {
  const service = readFileSync(coordinatorPath, 'utf8')

  assert.match(
    service,
    /export interface AdvancedConfig[\s\S]*ingress_countermeasure: IngressCountermeasureConfig/,
    'AdvancedConfig should include ingress_countermeasure aggregate field',
  )
  assert.match(
    service,
    /export interface IngressCountermeasureConfig[\s\S]*classifierThresholds: ClassifierThresholds[\s\S]*personaProfiles: PersonaProfile\[\][\s\S]*deceptionMode: DeceptionMode[\s\S]*responseDelayRanges: ResponseDelayRanges[\s\S]*fakeSurfacePolicies: FakeSurfacePolicy\[\][\s\S]*egressStabilitySupport: EgressStabilitySupportConfig/,
    'IngressCountermeasureConfig should define the expected contract fields',
  )
  assert.match(
    service,
    /export type PersonaTone = 'restrained' \| 'neutral' \| 'helpful'/,
    'PersonaTone should match Rust serde camelCase enum values',
  )
  assert.match(
    service,
    /export type SurfaceBias = 'decoy' \| 'balanced' \| 'production'/,
    'SurfaceBias should match Rust serde camelCase enum values',
  )
  assert.match(
    service,
    /export type DeceptionMode =[\s\S]*'disabled'[\s\S]*'observeOnly'[\s\S]*'decoyPreferred'[\s\S]*'decoyOnly'/,
    'DeceptionMode should match Rust serde camelCase enum values',
  )
  assert.match(
    service,
    /function normalizeAdvancedConfig[\s\S]*normalizeIpReputationConfig/,
    'aggregate AdvancedConfig reads should continue using normalizeAdvancedConfig',
  )
  assert.match(
    service,
    /function serializeAdvancedConfig[\s\S]*serializeIpReputationConfig/,
    'aggregate AdvancedConfig writes should continue using serializeAdvancedConfig',
  )
  assert.match(
    service,
    /getAdvancedConfig[\s\S]*normalizeAdvancedConfig/,
    'aggregate AdvancedConfig loads should normalize before panels read config',
  )
  assert.match(
    service,
    /saveAdvancedConfig[\s\S]*serializeAdvancedConfig/,
    'aggregate AdvancedConfig saves should serialize before Tauri receives config',
  )
  assert.match(
    service,
    /validateAdvancedConfig[\s\S]*serializeAdvancedConfig/,
    'aggregate AdvancedConfig validation should use the same backend contract as saves',
  )
  assert.doesNotMatch(
    service,
    /function normalizeIngressCountermeasureConfig|function serializeIngressCountermeasureConfig/,
    'ingress countermeasure should not add redundant identity-only boundary helpers',
  )
})
