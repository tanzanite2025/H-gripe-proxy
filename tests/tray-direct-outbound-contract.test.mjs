import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const trayMenuDefPath = join(
  repoRoot,
  'src-tauri',
  'src',
  'core',
  'tray',
  'menu_def.rs',
)
const trayModPath = join(repoRoot, 'src-tauri', 'src', 'core', 'tray', 'mod.rs')
const backendLocalesPath = join(repoRoot, 'crates', 'clash-verge-i18n', 'locales')
const frontendLocalesPath = join(repoRoot, 'src', 'locales')

test('tray outbound mode menu only exposes proxy-chain modes', () => {
  const menuDef = readFileSync(trayMenuDefPath, 'utf8')
  const trayMod = readFileSync(trayModPath, 'utf8')

  assert.match(menuDef, /rule_mode => RULE_MODE/)
  assert.match(menuDef, /global_mode => GLOBAL_MODE/)
  assert.doesNotMatch(menuDef, /direct_mode => DIRECT_MODE/)
  assert.doesNotMatch(menuDef, /tray_direct_mode/)
  assert.doesNotMatch(menuDef, /tray\.directMode/)

  assert.doesNotMatch(trayMod, /let direct_mode = &CheckMenuItem::with_id/)
  assert.doesNotMatch(trayMod, /MenuIds::DIRECT_MODE/)
  assert.doesNotMatch(trayMod, /hotkeys\.get\("clash_mode_direct"\)/)
})

test('tray resolves visible proxy-chain mode from the public clash-mode parser', () => {
  const trayMod = readFileSync(trayModPath, 'utf8')

  assert.match(
    trayMod,
    /let current_proxy_mode = match mode\.and_then\(\|value\| value\.parse::<ClashMode>\(\)\.ok\(\)\) \{/,
  )
  assert.match(trayMod, /Some\(ClashMode::Global\) => "global"/)
  assert.doesNotMatch(trayMod, /normalize_proxy_chain_mode/)
  assert.doesNotMatch(trayMod, /"direct" => "rule"/)
})

test('tray locale resources do not keep direct outbound mode copy', () => {
  for (const localeName of readdirSync(backendLocalesPath)) {
    const localeSource = readFileSync(
      join(backendLocalesPath, localeName),
      'utf8',
    )

    assert.doesNotMatch(
      localeSource,
      /^\s+directMode:/m,
      `${localeName} should not define tray.directMode`,
    )
    assert.doesNotMatch(
      localeSource,
      /^\s+direct:/m,
      `${localeName} should not define tray.direct`,
    )
  }
})

test('tray visible copy names proxy-chain modes instead of outbound modes', () => {
  const enBackendLocale = readFileSync(
    join(backendLocalesPath, 'en.yml'),
    'utf8',
  )
  const zhBackendLocale = readFileSync(
    join(backendLocalesPath, 'zh.yml'),
    'utf8',
  )

  assert.match(enBackendLocale, /^\s+outboundModes: Proxy-chain Modes$/m)
  assert.match(zhBackendLocale, /^\s+outboundModes:/m)
})

test('tray outbound mode display is fixed instead of configurable from layout settings', () => {
  const trayMod = readFileSync(trayModPath, 'utf8')

  assert.match(trayMod, /let show_outbound_modes_inline = false/)
  assert.doesNotMatch(trayMod, /verge(?:_settings)?\.tray_inline_outbound_modes/)

  for (const localeName of readdirSync(frontendLocalesPath)) {
    const localeSource = readFileSync(
      join(frontendLocalesPath, localeName, 'settings.json'),
      'utf8',
    )

    assert.doesNotMatch(localeSource, /showOutboundModesInline/, localeName)
    assert.doesNotMatch(localeSource, /"layout"\s*:/, localeName)
  }
})
