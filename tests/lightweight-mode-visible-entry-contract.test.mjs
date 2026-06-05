import assert from 'node:assert/strict'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const read = (...segments) => readFileSync(join(repoRoot, ...segments), 'utf8')
const exists = (...segments) => existsSync(join(repoRoot, ...segments))

test('homepage no longer exposes a lightweight mode entry action', () => {
  const source = read('src', 'pages', 'home.tsx')

  assert.doesNotMatch(source, /entry_lightweight_mode/)
  assert.doesNotMatch(source, /home\.page\.tooltips\.lightweightMode/)
})

test('settings pages no longer expose the lightweight mode settings dialog', () => {
  for (const file of [
    ['src', 'components', 'setting', 'setting-verge-basic.tsx'],
    ['src', 'components', 'setting', 'setting-verge-advanced.tsx'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /LiteModeViewer/)
    assert.doesNotMatch(source, /liteModeRef/)
    assert.doesNotMatch(source, /liteModeSettings/)
  }

  assert.equal(
    exists('src', 'components', 'setting', 'components', 'misc', 'lite-mode.tsx'),
    false,
  )
})

test('hotkey settings no longer expose or preserve lightweight shortcut entries', () => {
  const source = read(
    'src',
    'components',
    'setting',
    'components',
    'hotkey',
    'hotkey-config.tsx',
  )

  assert.doesNotMatch(source, /entry_lightweight_mode/)
  assert.doesNotMatch(source, /entryLightweightMode/)
  assert.match(source, /if \(!isHotkeyFunction\(func\)\) return/)
  assert.match(source, /\.filter\(\(\[func\]\) => isHotkeyFunction\(func\)\)/)
})

test('frontend command wrappers no longer expose lightweight mode commands', () => {
  const source = read('src', 'services', 'cmds.ts')

  assert.doesNotMatch(source, /entry_lightweight_mode/)
  assert.doesNotMatch(source, /exit_lightweight_mode/)
})

test('frontend locale resources no longer keep lightweight mode copy', () => {
  for (const localeName of readdirSync(join(repoRoot, 'src', 'locales'))) {
    const localeDir = join(repoRoot, 'src', 'locales', localeName)
    if (!existsSync(localeDir)) continue

    for (const fileName of ['home.json', 'settings.json']) {
      const source = read('src', 'locales', localeName, fileName)

      assert.doesNotMatch(source, /lightweightMode/, `${localeName}/${fileName}`)
      assert.doesNotMatch(source, /liteModeSettings/, `${localeName}/${fileName}`)
      assert.doesNotMatch(source, /"liteMode"\s*:/, `${localeName}/${fileName}`)
      assert.doesNotMatch(source, /entryLightweightMode/, `${localeName}/${fileName}`)
    }
  }
})

test('generated i18n types no longer include lightweight mode keys', () => {
  for (const file of [
    ['src', 'types', 'generated', 'i18n-keys.ts'],
    ['src', 'types', 'generated', 'i18n-resources.ts'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /lightweightMode/)
    assert.doesNotMatch(source, /liteModeSettings/)
    assert.doesNotMatch(source, /entryLightweightMode/)
    assert.doesNotMatch(source, /liteMode/)
  }
})

test('backend hotkey handling ignores stale lightweight shortcut configs', () => {
  const source = read('src-tauri', 'src', 'core', 'hotkey.rs')

  assert.match(source, /StaleEntryLightweightMode/)
  assert.match(source, /"entry_lightweight_mode" => Ok\(Self::StaleEntryLightweightMode\)/)
  assert.match(source, /HotkeyFunction::StaleEntryLightweightMode => \{/)
  assert.doesNotMatch(source, /entry_lightweight_mode\(\)\.await/)
})

test('tray menu no longer exposes a lightweight mode toggle', () => {
  const menuDef = read('src-tauri', 'src', 'core', 'tray', 'menu_def.rs')
  const trayMod = read('src-tauri', 'src', 'core', 'tray', 'mod.rs')

  assert.doesNotMatch(menuDef, /LIGHTWEIGHT_MODE/)
  assert.doesNotMatch(menuDef, /tray_lightweight_mode/)
  assert.doesNotMatch(menuDef, /tray\.lightweightMode/)

  assert.doesNotMatch(trayMod, /let lightweight_mode = &CheckMenuItem::with_id/)
  assert.doesNotMatch(trayMod, /MenuIds::LIGHTWEIGHT_MODE/)
})

test('startup no longer auto-enters lightweight mode from stale config', () => {
  const resolveMod = read('src-tauri', 'src', 'utils', 'resolve', 'mod.rs')

  assert.doesNotMatch(resolveMod, /auto_lightweight_boot/)
  assert.doesNotMatch(resolveMod, /init_auto_lightweight_boot/)
})

test('backend command surface no longer registers lightweight mode commands', () => {
  const cmdMod = read('src-tauri', 'src', 'cmd', 'mod.rs')
  const appLib = read('src-tauri', 'src', 'lib.rs')

  assert.equal(exists('src-tauri', 'src', 'cmd', 'lightweight.rs'), false)
  assert.doesNotMatch(cmdMod, /pub mod lightweight/)
  assert.doesNotMatch(cmdMod, /pub use lightweight::\*/)
  assert.doesNotMatch(appLib, /cmd::entry_lightweight_mode/)
  assert.doesNotMatch(appLib, /cmd::exit_lightweight_mode/)
})

test('stale lightweight config fields are no longer accepted by typed config', () => {
  for (const file of [
    ['src', 'types', 'global.d.ts'],
    ['src-tauri', 'src', 'config', 'verge.rs'],
    ['src-tauri', 'src', 'feat', 'config.rs'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /enable_auto_light_weight_mode/)
    assert.doesNotMatch(source, /auto_light_weight_minutes/)
    assert.doesNotMatch(source, /LIGHT_WEIGHT/)
  }
})

test('lightweight runtime module and recovery hooks are removed', () => {
  const moduleMod = read('src-tauri', 'src', 'module', 'mod.rs')

  assert.equal(exists('src-tauri', 'src', 'module', 'lightweight.rs'), false)
  assert.doesNotMatch(moduleMod, /pub mod lightweight/)

  for (const file of [
    ['src-tauri', 'src', 'core', 'tray', 'mod.rs'],
    ['src-tauri', 'src', 'feat', 'window.rs'],
    ['src-tauri', 'src', 'utils', 'server.rs'],
    ['src-tauri', 'src', 'lib.rs'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /module::lightweight/)
    assert.doesNotMatch(source, /lightweight::/)
    assert.doesNotMatch(source, /is_in_lightweight_mode/)
    assert.doesNotMatch(source, /exit_lightweight_mode/)
  }
})

test('backend notification resources no longer mention lightweight mode', () => {
  const notification = read('src-tauri', 'src', 'utils', 'notification.rs')

  assert.doesNotMatch(notification, /LightweightModeEntered/)
  assert.doesNotMatch(notification, /lightweightModeEntered/)

  for (const localeName of readdirSync(join(repoRoot, 'crates', 'clash-verge-i18n', 'locales'))) {
    const source = read('crates', 'clash-verge-i18n', 'locales', localeName)

    assert.doesNotMatch(source, /lightweightModeEntered/, localeName)
  }
})
