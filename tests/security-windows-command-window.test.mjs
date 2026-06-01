import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()

test('security status checks use hidden Windows commands', () => {
  const antiDebug = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'security', 'anti_debug.rs'),
    'utf8',
  )
  const memoryHoneypot = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'security', 'memory_honeypot.rs'),
    'utf8',
  )

  assert.match(antiDebug, /hidden_command\("wmic"\)/)
  assert.doesNotMatch(antiDebug, /Command::new\("wmic"\)/)

  assert.match(memoryHoneypot, /hidden_command\("tasklist"\)/)
  assert.doesNotMatch(memoryHoneypot, /Command::new\("tasklist"\)/)
})

test('security action commands are hidden on Windows', () => {
  const firewall = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'security', 'firewall.rs'),
    'utf8',
  )
  const localSecurity = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'security', 'local_security.rs'),
    'utf8',
  )
  const localStealth = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'security', 'local_stealth.rs'),
    'utf8',
  )

  for (const source of [firewall, localSecurity, localStealth]) {
    assert.doesNotMatch(
      source,
      /(?:std::process::)?Command::new\("(?:net|netsh|netstat|powershell|reg)"\)/,
    )
  }

  for (const command of ['net', 'powershell']) {
    assert.match(firewall, new RegExp(`hidden_command\\("${command}"\\)`))
  }
  assert.match(localSecurity, /hidden_command\("netstat"\)/)
  for (const command of ['net', 'netsh', 'reg']) {
    assert.match(localStealth, new RegExp(`hidden_command\\("${command}"\\)`))
  }
})
