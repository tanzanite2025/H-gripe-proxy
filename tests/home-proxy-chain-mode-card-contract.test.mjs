import assert from 'node:assert/strict'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
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
const localesPath = join(repoRoot, 'src', 'locales')
const i18nKeysPath = join(repoRoot, 'src', 'types', 'generated', 'i18n-keys.ts')
const i18nResourcesPath = join(
  repoRoot,
  'src',
  'types',
  'generated',
  'i18n-resources.ts',
)

test('home clash mode card has been physically removed', () => {
  assert.equal(existsSync(clashModeCardPath), false)
})

test('home locales do not keep stale clash-mode copy', () => {
  for (const localeName of readdirSync(localesPath)) {
    const homeLocalePath = join(localesPath, localeName, 'home.json')
    const homeLocale = JSON.parse(readFileSync(homeLocalePath, 'utf8'))

    assert.equal(
      'proxyMode' in (homeLocale.page?.cards || {}),
      false,
      `${localeName} home page cards should not keep proxyMode`,
    )
    assert.equal(
      'proxyMode' in (homeLocale.page?.settings?.cards || {}),
      false,
      `${localeName} home settings cards should not keep proxyMode`,
    )
    assert.equal(
      'globalMode' in (homeLocale.components?.currentProxy?.labels || {}),
      false,
      `${localeName} currentProxy labels should not keep globalMode`,
    )
    assert.equal(
      'clashMode' in (homeLocale.components || {}),
      false,
      `${localeName} home components should not keep clashMode`,
    )
  }
})

test('generated i18n types do not keep removed home clash-mode keys', () => {
  const keysSource = readFileSync(i18nKeysPath, 'utf8')
  const resourcesSource = readFileSync(i18nResourcesPath, 'utf8')

  assert.doesNotMatch(keysSource, /home\.page\.cards\.proxyMode/)
  assert.doesNotMatch(keysSource, /home\.page\.settings\.cards\.proxyMode/)
  assert.doesNotMatch(keysSource, /home\.components\.currentProxy\.labels\.globalMode/)
  assert.doesNotMatch(keysSource, /home\.components\.clashMode\./)
  assert.doesNotMatch(resourcesSource, /proxyMode: string/)
  assert.doesNotMatch(resourcesSource, /globalMode: string/)
  assert.doesNotMatch(resourcesSource, /clashMode:\s*\{/)
})
