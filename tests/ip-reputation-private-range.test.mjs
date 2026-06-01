import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const backendPath = join(repoRoot, 'src-tauri', 'src', 'core', 'ip_reputation.rs')

test('ip reputation fallback uses parsed private/reserved IP ranges', () => {
  const backend = readFileSync(backendPath, 'utf8')

  assert.match(backend, /use std::net::IpAddr;/)
  assert.match(backend, /fn is_private_or_reserved_ip\(/)
  assert.match(backend, /ip\.parse::<IpAddr>\(\)/)
  assert.match(backend, /addr\.is_private\(\)/)
  assert.match(backend, /addr\.is_loopback\(\)/)
  assert.match(backend, /addr\.is_link_local\(\)/)
  assert.match(backend, /addr\.is_unspecified\(\)/)
  assert.match(backend, /is_carrier_grade_nat_ip\(/)
  assert.match(backend, /100\.\.=127/)
  assert.match(backend, /manager\.detect_ip_type\("172\.20\.1\.1"\), IpType::Unknown/)
  assert.match(backend, /manager\.detect_ip_type\("100\.64\.1\.1"\), IpType::Unknown/)
  assert.doesNotMatch(backend, /ip\.starts_with\("172\.16\."\)/)
})
