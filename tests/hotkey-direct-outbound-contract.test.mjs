import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const hotkeyConfigPath = join(
  repoRoot,
  'src',
  'components',
  'setting',
  'components',
  'hotkey',
  'hotkey-config.tsx',
)
const backendHotkeyPath = join(repoRoot, 'src-tauri', 'src', 'core', 'hotkey.rs')
const localesPath = join(repoRoot, 'src', 'locales')

test('hotkey settings do not expose a global direct shortcut entry', () => {
  const source = readFileSync(hotkeyConfigPath, 'utf8')

  assert.doesNotMatch(source, /clash_mode_direct/)
  assert.doesNotMatch(source, /functions\.direct/)
})

test('backend hotkey handling ignores stale direct shortcut configs', () => {
  const source = readFileSync(backendHotkeyPath, 'utf8')

  assert.match(source, /StaleClashModeDirect/)
  assert.match(source, /"clash_mode_direct" => Ok\(Self::StaleClashModeDirect\)/)
  assert.match(source, /HotkeyFunction::StaleClashModeDirect => \{/)
  assert.doesNotMatch(source, /change_clash_mode\(ClashMode::Direct\)/)
  assert.doesNotMatch(source, /\bClashModeDirect\b/)
})

test('hotkey locale does not keep direct function copy', () => {
  for (const localeName of readdirSync(localesPath)) {
    const settingsLocalePath = join(localesPath, localeName, 'settings.json')
    const settingsLocale = JSON.parse(readFileSync(settingsLocalePath, 'utf8'))

    assert.equal(
      'direct' in settingsLocale.modals.hotkey.functions,
      false,
      `${localeName} hotkey functions should not keep direct copy`,
    )
  }
})
