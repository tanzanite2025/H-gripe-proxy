import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const trayMenuDefPath = join(repoRoot, 'src-tauri', 'src', 'core', 'tray', 'menu_def.rs')
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

test('tray direct runtime state is displayed as proxy-chain rule state', () => {
  const trayMod = readFileSync(trayModPath, 'utf8')

  assert.match(trayMod, /let current_proxy_mode = normalize_proxy_chain_mode\(mode\.unwrap_or\(""\)\)/)
  assert.match(trayMod, /fn normalize_proxy_chain_mode\(mode: &str\) -> &str/)
  assert.match(trayMod, /"direct" => "rule"/)
  assert.doesNotMatch(trayMod, /"direct" => clash_verge_i18n::t!\("tray\.direct"\)/)
})

test('tray locale resources do not keep direct outbound mode copy', () => {
  for (const localeName of readdirSync(backendLocalesPath)) {
    const localeSource = readFileSync(join(backendLocalesPath, localeName), 'utf8')

    assert.doesNotMatch(localeSource, /^\s+directMode:/m, `${localeName} should not define tray.directMode`)
    assert.doesNotMatch(localeSource, /^\s+direct:/m, `${localeName} should not define tray.direct`)
  }
})

test('tray visible copy names proxy-chain modes instead of outbound modes', () => {
  const enBackendLocale = readFileSync(join(backendLocalesPath, 'en.yml'), 'utf8')
  const zhBackendLocale = readFileSync(join(backendLocalesPath, 'zh.yml'), 'utf8')
  const enSettingsLocale = JSON.parse(readFileSync(join(frontendLocalesPath, 'en', 'settings.json'), 'utf8'))
  const zhSettingsLocale = JSON.parse(readFileSync(join(frontendLocalesPath, 'zh', 'settings.json'), 'utf8'))

  assert.match(enBackendLocale, /^\s+outboundModes: Proxy-chain Modes$/m)
  assert.match(zhBackendLocale, /^\s+outboundModes: 代理链路模式$/m)
  assert.equal(
    enSettingsLocale.components.verge.layout.fields.showOutboundModesInline,
    'Show Proxy-chain Modes Inline',
  )
  assert.equal(
    zhSettingsLocale.components.verge.layout.fields.showOutboundModesInline,
    '将代理链路模式显示在托盘一级菜单',
  )
})
