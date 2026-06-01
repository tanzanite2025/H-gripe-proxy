import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()

test('honeypot code lives behind a dedicated security::honeypot domain', () => {
  const honeypotDir = join(repoRoot, 'src-tauri', 'src', 'security', 'honeypot')
  const securityMod = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'security', 'mod.rs'),
    'utf8',
  )
  const runtime = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'core', 'security_runtime.rs'),
    'utf8',
  )
  const commands = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'cmd', 'security.rs'),
    'utf8',
  )

  assert.ok(existsSync(join(honeypotDir, 'mod.rs')))
  assert.ok(existsSync(join(honeypotDir, 'memory.rs')))
  assert.ok(existsSync(join(honeypotDir, 'decoy_file.rs')))
  assert.ok(existsSync(join(honeypotDir, 'secure_storage.rs')))

  assert.match(securityMod, /pub mod honeypot;/)
  assert.doesNotMatch(securityMod, /pub mod memory_honeypot;/)
  assert.doesNotMatch(securityMod, /pub mod config_decoy;/)

  assert.match(runtime, /crate::security::honeypot/)
  assert.doesNotMatch(runtime, /memory_honeypot/)
  assert.doesNotMatch(runtime, /config_decoy/)

  assert.match(commands, /crate::security::honeypot::HoneypotStats/)
  assert.match(commands, /honeypot::get_global_honeypot_stats/)
  assert.match(commands, /honeypot::check_global_honeypot/)
})
