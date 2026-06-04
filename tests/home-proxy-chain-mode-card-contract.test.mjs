import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const clashModeCardPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'clash-mode-card.tsx',
)
const zhHomeLocalePath = join(repoRoot, 'src', 'locales', 'zh', 'home.json')
const enHomeLocalePath = join(repoRoot, 'src', 'locales', 'en', 'home.json')
const localesPath = join(repoRoot, 'src', 'locales')
const i18nKeysPath = join(repoRoot, 'src', 'types', 'generated', 'i18n-keys.ts')
const i18nResourcesPath = join(repoRoot, 'src', 'types', 'generated', 'i18n-resources.ts')

test('home clash mode card only exposes proxy-chain rule and global modes', () => {
  const source = readFileSync(clashModeCardPath, 'utf8')

  assert.match(source, /const HOME_PROXY_CHAIN_MODES = \['rule', 'global'\] as const/)
  assert.match(source, /type HomeProxyChainMode = \(typeof HOME_PROXY_CHAIN_MODES\)\[number\]/)
  assert.match(source, /label: '应用规则'/)
  assert.match(source, /label: '不应用规则'/)
  assert.match(source, /HOME_PROXY_CHAIN_MODES\.map\(\(mode\)/)
  assert.match(source, /const displayMode =[\s\S]*?optimisticMode \?\? \(currentModeKey === 'direct' \? 'rule' : currentModeKey\)/)
  assert.doesNotMatch(source, /CLASH_MODES/)
  assert.doesNotMatch(source, /direct:\s*\{/)
  assert.doesNotMatch(source, /labels\.direct/)
  assert.doesNotMatch(source, /<Send\b/)
})

test('home mode locale does not keep stale direct page copy', () => {
  const zh = JSON.parse(readFileSync(zhHomeLocalePath, 'utf8'))
  const en = JSON.parse(readFileSync(enHomeLocalePath, 'utf8'))

  assert.equal('direct' in zh.components.clashMode.labels, false)
  assert.equal('direct' in zh.components.clashMode.descriptions, false)
  assert.equal('directMode' in zh.components.currentProxy.labels, false)
  assert.equal('direct' in en.components.clashMode.labels, false)
  assert.equal('direct' in en.components.clashMode.descriptions, false)
  assert.equal('directMode' in en.components.currentProxy.labels, false)

  for (const localeName of readdirSync(localesPath)) {
    const homeLocalePath = join(localesPath, localeName, 'home.json')
    const homeLocale = JSON.parse(readFileSync(homeLocalePath, 'utf8'))

    assert.equal(
      'direct' in homeLocale.components.clashMode.labels,
      false,
      `${localeName} home clashMode labels should not keep direct`,
    )
    assert.equal(
      'direct' in homeLocale.components.clashMode.descriptions,
      false,
      `${localeName} home clashMode descriptions should not keep direct`,
    )
    assert.equal(
      'directMode' in homeLocale.components.currentProxy.labels,
      false,
      `${localeName} currentProxy labels should not keep directMode`,
    )
  }
})

test('generated i18n types do not keep stale home direct mode keys', () => {
  const keysSource = readFileSync(i18nKeysPath, 'utf8')
  const resourcesSource = readFileSync(i18nResourcesPath, 'utf8')

  assert.doesNotMatch(keysSource, /home\.components\.clashMode\.labels\.direct/)
  assert.doesNotMatch(keysSource, /home\.components\.clashMode\.descriptions\.direct/)
  assert.doesNotMatch(keysSource, /home\.components\.currentProxy\.labels\.directMode/)
  assert.doesNotMatch(resourcesSource, /clashMode:\s*\{[\s\S]*?descriptions:\s*\{[^}]*direct: string/)
  assert.doesNotMatch(resourcesSource, /currentProxy:\s*\{[\s\S]*?labels:\s*\{[^}]*directMode: string/)
})
