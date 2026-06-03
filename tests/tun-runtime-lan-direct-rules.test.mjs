import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const tunEnhancePath = join(repoRoot, 'src-tauri', 'src', 'enhance', 'tun.rs')

test('TUN runtime injects private LAN DIRECT rules before MATCH fallback', () => {
  const tunEnhance = readFileSync(tunEnhancePath, 'utf8')

  assert.match(tunEnhance, /const LAN_DIRECT_RULES: \[&str; \d+\]/)
  assert.match(tunEnhance, /IP-CIDR,10\.0\.0\.0\/8,DIRECT,no-resolve/)
  assert.match(tunEnhance, /IP-CIDR,172\.16\.0\.0\/12,DIRECT,no-resolve/)
  assert.match(tunEnhance, /IP-CIDR,192\.168\.0\.0\/16,DIRECT,no-resolve/)
  assert.match(tunEnhance, /fn ensure_lan_direct_rules_before_match\(/)
  assert.match(tunEnhance, /ensure_lan_direct_rules_before_match\(&mut config\)/)
  assert.match(tunEnhance, /position\(\|rule\| is_match_rule\(rule\)\)/)
  assert.match(tunEnhance, /seq\.insert\(insert_at, Value::from\(\*rule\)\)/)
})
