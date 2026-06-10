import assert from 'node:assert/strict'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
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
const proxyGroupsPath = join(
  repoRoot,
  'src',
  'components',
  'proxy',
  'proxy-groups',
  'hooks',
  'use-proxy-groups.ts',
)
const renderListPath = join(
  repoRoot,
  'src',
  'components',
  'proxy',
  'use-render-list.ts',
)
const renderListRuntimePath = join(
  repoRoot,
  'src',
  'components',
  'proxy',
  'use-render-list-runtime.ts',
)
const runtimeSummaryPath = join(
  repoRoot,
  'src',
  'components',
  'proxy',
  'use-runtime-summary-item.ts',
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

test('proxy page mode helper file has been physically removed', () => {
  assert.equal(existsSync(proxySharedPath), false)
})

test('proxy page hooks no longer carry a fake display mode parameter', () => {
  const controllerSource = readFileSync(proxyGroupsControllerPath, 'utf8')
  const groupsSource = readFileSync(proxyGroupsPath, 'utf8')
  const renderListSource = readFileSync(renderListPath, 'utf8')
  const renderListRuntimeSource = readFileSync(renderListRuntimePath, 'utf8')
  const runtimeSummarySource = readFileSync(runtimeSummaryPath, 'utf8')

  assert.doesNotMatch(controllerSource, /displayMode|mode: displayMode/)
  assert.doesNotMatch(groupsSource, /mode: string|const \{ mode,/)
  assert.doesNotMatch(renderListSource, /mode: string|useRenderList = \(mode/)
  assert.doesNotMatch(renderListRuntimeSource, /mode: string|useRuntimeSummaryItem\(mode\)|mode,/)
  assert.doesNotMatch(runtimeSummarySource, /useRuntimeSummaryItem = \(mode/)
})

test('proxy page locales do not keep removed page mode copy', () => {
  for (const localeName of readdirSync(localesPath)) {
    const proxyLocalePath = join(localesPath, localeName, 'proxies.json')
    const proxyLocale = JSON.parse(readFileSync(proxyLocalePath, 'utf8'))

    assert.equal(
      'modes' in (proxyLocale.page || {}),
      false,
      `${localeName} proxies page should not keep page.modes`,
    )
  }
})

test('generated i18n types do not keep removed proxy page mode keys', () => {
  const keysSource = readFileSync(i18nKeysPath, 'utf8')
  const resourcesSource = readFileSync(i18nResourcesPath, 'utf8')

  assert.doesNotMatch(keysSource, /proxies\.page\.modes\./)
  assert.doesNotMatch(resourcesSource, /modes:\s*\{/)
})
