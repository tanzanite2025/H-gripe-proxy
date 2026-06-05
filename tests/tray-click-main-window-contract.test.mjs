import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const read = (...segments) => readFileSync(join(repoRoot, ...segments), 'utf8')

test('basic settings no longer exposes tray click behavior', () => {
  const source = read('src', 'components', 'setting', 'setting-verge-basic.tsx')

  assert.doesNotMatch(source, /tray_event/)
  assert.doesNotMatch(source, /trayClickEvent/)
  assert.doesNotMatch(source, /trayOptions/)
})

test('typed verge config no longer accepts tray click behavior', () => {
  for (const file of [
    ['src', 'types', 'global.d.ts'],
    ['src-tauri', 'src', 'config', 'verge.rs'],
    ['src-tauri', 'src', 'feat', 'config.rs'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /tray_event/)
    assert.doesNotMatch(source, /SYSTRAY_CLICK_BEHAVIOR/)
    assert.doesNotMatch(source, /update_click_behavior/)
  }
})

test('tray left click is fixed to showing the main window', () => {
  const menuDef = read('src-tauri', 'src', 'core', 'tray', 'menu_def.rs')
  const trayMod = read('src-tauri', 'src', 'core', 'tray', 'mod.rs')

  assert.doesNotMatch(menuDef, /TrayAction/)
  assert.doesNotMatch(trayMod, /TrayAction/)
  assert.doesNotMatch(trayMod, /let show_menu_on_left_click/)
  assert.doesNotMatch(trayMod, /set_show_menu_on_left_click/)
  assert.doesNotMatch(trayMod, /verge_tray_event/)
  assert.doesNotMatch(trayMod, /TrayAction::SystemProxy/)
  assert.doesNotMatch(trayMod, /TrayAction::TunMode/)
  assert.match(trayMod, /builder = builder\.show_menu_on_left_click\(false\);/)
  assert.match(trayMod, /WindowManager::show_main_window\(\)\.await;/)
})

test('frontend locale resources no longer keep tray click behavior copy', () => {
  for (const localeName of readdirSync(join(repoRoot, 'src', 'locales'))) {
    const source = read('src', 'locales', localeName, 'settings.json')

    assert.doesNotMatch(source, /trayClickEvent/, localeName)
    assert.doesNotMatch(source, /trayOptions/, localeName)
    assert.doesNotMatch(source, /showMainWindow/, localeName)
    assert.doesNotMatch(source, /showTrayMenu/, localeName)
  }
})

test('generated i18n types no longer include tray click behavior keys', () => {
  for (const file of [
    ['src', 'types', 'generated', 'i18n-keys.ts'],
    ['src', 'types', 'generated', 'i18n-resources.ts'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /trayClickEvent/)
    assert.doesNotMatch(source, /trayOptions/)
    assert.doesNotMatch(source, /showMainWindow/)
    assert.doesNotMatch(source, /showTrayMenu/)
  }
})
