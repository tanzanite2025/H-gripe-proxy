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
  assert.match(shell, /honeypotDecoys=\{honeypotDecoys\}/)
  assert.match(shell, /activeDecoyId=\{activeDecoyId\}/)
  assert.match(shell, /onActiveDecoyChange=\{onActiveDecoyChange\}/)
  assert.match(shell, /onAddHoneypotDecoy=\{onAddHoneypotDecoy\}/)
  assert.match(shell, /onRemoveHoneypotDecoy=\{onRemoveHoneypotDecoy\}/)
  assert.match(shell, /onHoneypotDecoyEnabledChange=\{onHoneypotDecoyEnabledChange\}/)
  assert.match(shell, /onApplyHoneypotDecoyStrategy=\{onApplyHoneypotDecoyStrategy\}/)
  assert.match(shell, /SecureStoragePanel/)
  assert.match(shell, /SelfDestructPanel/)
  assert.doesNotMatch(shell, /status\.debugger_present/)
  assert.doesNotMatch(shell, /securityGenerateEncryptionKey/)
})

test('honeypot decoy panel exposes dynamic decoy controls', () => {
  const panel = readFileSync(
    join(panelDir, 'honeypot-decoy-panel.tsx'),
    'utf8',
  )

  assert.match(panel, /honeypotDecoys/)
  assert.match(panel, /activeDecoyId/)
  assert.match(panel, /onActiveDecoyChange/)
  assert.match(panel, /onAddHoneypotDecoy/)
  assert.match(panel, /onRemoveHoneypotDecoy/)
  assert.match(panel, /onHoneypotDecoyEnabledChange/)
  assert.match(panel, /onApplyHoneypotDecoyStrategy/)
  assert.match(panel, /Switch/)
  assert.match(panel, /Plus/)
  assert.match(panel, /Trash2/)
  assert.match(panel, /Sparkles/)
})
