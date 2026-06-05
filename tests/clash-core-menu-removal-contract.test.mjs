import assert from 'node:assert/strict'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const read = (...segments) => readFileSync(join(repoRoot, ...segments), 'utf8')
const exists = (...segments) => existsSync(join(repoRoot, ...segments))

test('clash settings no longer exposes the Clash core settings dialog', () => {
  const source = read('src', 'components', 'setting', 'setting-clash.tsx')

  assert.doesNotMatch(source, /ClashCoreViewer/)
  assert.doesNotMatch(source, /coreRef/)
  assert.doesNotMatch(source, /settings\.sections\.clash\.form\.fields\.clashCore/)
})

test('Clash core settings component is removed', () => {
  assert.equal(exists('src', 'components', 'setting', 'components', 'clash', 'clash-core.tsx'), false)
})

test('frontend no longer exposes a Clash core switch command', () => {
  const cmds = read('src', 'services', 'cmds.ts')
  const notificationHandlers = read('src', 'pages', '_layout', 'utils', 'notification-handlers.ts')

  assert.doesNotMatch(cmds, /changeClashCore/)
  assert.doesNotMatch(cmds, /change_clash_core/)
  assert.doesNotMatch(notificationHandlers, /config_core::change_/)
})

test('backend no longer registers or implements a Clash core switch command', () => {
  for (const file of [
    ['src-tauri', 'src', 'cmd', 'clash.rs'],
    ['src-tauri', 'src', 'feat', 'clash.rs'],
    ['src-tauri', 'src', 'core', 'manager', 'lifecycle.rs'],
    ['src-tauri', 'src', 'lib.rs'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /change_clash_core/)
    assert.doesNotMatch(source, /change_core/)
    assert.doesNotMatch(source, /config_core::change_/)
  }
})

test('fixed local mihomo sidecar remains the only valid core path', () => {
  const vergeConfig = read('src-tauri', 'src', 'config', 'verge.rs')
  const tauriConfig = read('src-tauri', 'tauri.conf.json')

  assert.match(vergeConfig, /VALID_CLASH_CORES[^=]*=\s*&\["verge-mihomo"\]/)
  assert.match(vergeConfig, /clash_core:\s+Some\("verge-mihomo"\.into\(\)\)/)
  assert.match(tauriConfig, /"sidecar\/verge-mihomo"/)
})

test('frontend locale resources no longer keep Clash core menu copy', () => {
  for (const localeName of readdirSync(join(repoRoot, 'src', 'locales'))) {
    const source = read('src', 'locales', localeName, 'settings.json')

    assert.doesNotMatch(source, /"clashCore"/, localeName)
  }
})

test('generated i18n types no longer include Clash core menu keys', () => {
  for (const file of [
    ['src', 'types', 'generated', 'i18n-keys.ts'],
    ['src', 'types', 'generated', 'i18n-resources.ts'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /clashCore/)
  }
})
