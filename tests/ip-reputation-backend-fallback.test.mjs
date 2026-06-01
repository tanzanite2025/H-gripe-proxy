import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const backendPath = join(repoRoot, 'src-tauri', 'src', 'core', 'ip_reputation.rs')

test('ip reputation backend uses prefix fallback when ASN classification is unknown', () => {
  const backend = readFileSync(backendPath, 'utf8')

  assert.match(backend, /fn resolve_ip_type\(/)
  assert.match(backend, /asn_ip_type == IpType::Unknown/)
  assert.match(backend, /self\.detect_ip_type\(ip\)/)
  assert.match(
    backend,
    /let ip_type = self\.resolve_ip_type\(ip, &asn_info\);/,
    'check_ip_local should use fallback-aware classification instead of direct ASN conversion',
  )
  assert.doesNotMatch(
    backend,
    /let ip_type = IpType::from\(asn_info\.category\);/,
    'direct ASN conversion leaves unknown classifications disconnected from prefix fallback',
  )
})
