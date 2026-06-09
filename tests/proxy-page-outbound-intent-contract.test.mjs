import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const proxySharedPath = join(
  repoRoot,
  'src',
  'pages',
  'proxies-page',
  'shared.ts',
)
const proxyGroupsControllerPath = join(
  repoRoot,
  'src',
  'components',
  'proxy',
  'proxy-groups',
  'hooks',
  'use-proxy-groups-controller.ts',
)
const localesPath = join(repoRoot, 'src', 'locales')
const i18nKeysPath = join(repoRoot, 'src', 'types', 'generated', 'i18n-keys.ts')
const i18nResourcesPath = join(repoRoot, 'src', 'types', 'generated', 'i18n-resources.ts')

test('proxy page only exposes proxy-chain rule and global modes', () => {
  const source = readFileSync(proxySharedPath, 'utf8')

  assert.match(source, /const PROXY_CHAIN_MODES = \['rule', 'global'\] as const/)
  assert.match(source, /type ProxyChainMode = \(typeof PROXY_CHAIN_MODES\)\[number\]/)
  assert.doesNotMatch(source, /'direct'/)
})

test('proxy groups do not keep a direct-to-rule display shim', () => {
  const source = readFileSync(proxyGroupsControllerPath, 'utf8')

  assert.match(source, /const displayMode = mode/)
  assert.match(source, /useChainMode\(\{[\s\S]*mode: displayMode/)
  assert.match(source, /useProxyGroups\(\{[\s\S]*mode: displayMode/)
  assert.doesNotMatch(source, /mode === 'direct'/)
})

test('proxy page locale does not keep stale direct mode copy', () => {
  const localeNames = readdirSync(localesPath)

  for (const localeName of localeNames) {
    const proxyLocalePath = join(localesPath, localeName, 'proxies.json')
    const proxyLocale = JSON.parse(readFileSync(proxyLocalePath, 'utf8'))

    assert.equal(
      'direct' in proxyLocale.page.modes,
      false,
      `${localeName} proxies page should not keep modes.direct`,
    )
    assert.equal(
      'directMode' in proxyLocale.page.messages,
      false,
      `${localeName} proxies page should not keep messages.directMode`,
    )
  }
})

test('generated i18n types do not keep stale proxy page direct mode keys', () => {
  const keysSource = readFileSync(i18nKeysPath, 'utf8')
  const resourcesSource = readFileSync(i18nResourcesPath, 'utf8')

  assert.doesNotMatch(keysSource, /proxies\.page\.modes\.direct/)
  assert.doesNotMatch(resourcesSource, /proxies:\s*\{[\s\S]*?page:\s*\{[\s\S]*?modes:\s*\{[^}]*direct: string/)
})
