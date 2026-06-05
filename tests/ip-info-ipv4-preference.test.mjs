import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const apiServicePath = join(repoRoot, 'src', 'services', 'api.ts')
const componentPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'ip-info-card.tsx',
)

test('ip info service and header prefer IPv4 for compact display', () => {
  const apiService = readFileSync(apiServicePath, 'utf8')
  const component = readFileSync(componentPath, 'utf8')

  assert.match(
    apiService,
    /const isIPv4Address = \(value: unknown\): value is string/,
  )
  assert.match(apiService, /const pickPreferredIpInfo = <T extends IpInfo/)
  assert.match(apiService, /candidate\.ip\.includes\(':'\)/)
  assert.match(apiService, /plainIpSources/)
  assert.match(apiService, /https:\/\/api\.ipify\.org\?format=json/)
  assert.match(apiService, /https:\/\/api4\.ipify\.org\?format=json/)
  assert.match(
    apiService,
    /pickPreferredIpInfo\(\[\s*ipInfo,\s*ipv4Info\s*\]\)/,
  )
  assert.match(
    apiService,
    /pickPreferredIpInfo\(\[\s*Object\.assign\(service\.mapping\(data\)/,
  )

  assert.match(component, /const selectDisplayIp = \(/)
  assert.match(component, /public_egress_ip/)
  assert.match(component, /egress_ip/)
  assert.match(component, /ipInfo\?\.ip/)
  assert.match(component, /find\(isIPv4Address\)/)
})
