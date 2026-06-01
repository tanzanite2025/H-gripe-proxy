import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const securityComponents = join(repoRoot, 'src', 'components', 'security')
const panelDir = join(securityComponents, 'security-panels')

test('security monitor UI is composed from focused panel components', () => {
  const shell = readFileSync(join(securityComponents, 'security-monitor-ui.tsx'), 'utf8')
  const expectedPanels = [
    'security-status-panel.tsx',
    'honeypot-decoy-panel.tsx',
    'secure-storage-panel.tsx',
    'self-destruct-panel.tsx',
  ]

  for (const panel of expectedPanels) {
    assert.ok(existsSync(join(panelDir, panel)), `${panel} should exist`)
  }

  assert.match(shell, /SecurityStatusPanel/)
  assert.match(shell, /HoneypotDecoyPanel/)
  assert.match(shell, /SecureStoragePanel/)
  assert.match(shell, /SelfDestructPanel/)
  assert.doesNotMatch(shell, /status\.debugger_present/)
  assert.doesNotMatch(shell, /securityGenerateEncryptionKey/)
})
