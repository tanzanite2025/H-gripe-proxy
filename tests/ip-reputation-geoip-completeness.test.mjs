import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const geoipPath = join(
  repoRoot,
  'src-tauri',
  'src',
  'core',
  'runtime_diagnostics',
  'geoip.rs',
)

test('ip reputation geoip lookup prefers responses with ASN metadata', () => {
  const geoip = readFileSync(geoipPath, 'utf8')

  assert.match(geoip, /fn has_location_identity\(/)
  assert.match(geoip, /fn has_asn_metadata\(/)
  assert.match(geoip, /let mut fallback: Option<GeoIpInfo>/)
  assert.match(geoip, /if has_location_identity\(&info\) && has_asn_metadata\(&info\)/)
  assert.match(geoip, /fallback\.get_or_insert\(info\)/)
  assert.doesNotMatch(
    geoip,
    /if info\.country\.is_some\(\) \{\s*return info;\s*\}/,
    'returning the first country-only response can starve ASN classification',
  )
})
