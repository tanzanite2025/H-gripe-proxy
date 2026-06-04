import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const panelPath = join(repoRoot, 'src', 'components', 'advanced', 'dns-advanced-panel.tsx')

test('dns advanced panel groups long content behind internal tabs', () => {
  const source = readFileSync(panelPath, 'utf8')

  assert.match(source, /import \{ Tab, Tabs \} from '@\/components\/tailwind'/)
  assert.match(source, /const DNS_TAB_OVERVIEW = 'overview'/)
  assert.match(source, /const DNS_TAB_STATS = 'stats'/)
  assert.match(source, /const DNS_TAB_ROUTING = 'routing'/)
  assert.match(source, /const DNS_TAB_LEAK = 'leak'/)

  assert.match(source, /<Tab label="概览" value=\{DNS_TAB_OVERVIEW\}/)
  assert.match(source, /<Tab label="统计" value=\{DNS_TAB_STATS\}/)
  assert.match(source, /<Tab label="分流" value=\{DNS_TAB_ROUTING\}/)
  assert.match(source, /<Tab label="防泄漏" value=\{DNS_TAB_LEAK\}/)

  assert.match(source, /value=\{activeTab\}/)
  assert.match(source, /role="tabpanel"/)
})

test('dns runtime apply controls stay outside tab panels', () => {
  const source = readFileSync(panelPath, 'utf8')
  const runtimeCardIndex = source.indexOf('DNS 运行时应用')
  const tabsIndex = source.indexOf('<Tabs')

  assert.notEqual(runtimeCardIndex, -1, 'runtime apply card should remain present')
  assert.notEqual(tabsIndex, -1, 'internal tabs should be rendered')
  assert.ok(
    runtimeCardIndex < tabsIndex,
    'runtime apply controls should remain above internal tabs',
  )
})
