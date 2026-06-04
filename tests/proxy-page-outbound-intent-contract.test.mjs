import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const proxiesPagePath = join(repoRoot, 'src', 'pages', 'proxies.tsx')
const proxyGroupsPath = join(repoRoot, 'src', 'components', 'proxy', 'proxy-groups', 'index.tsx')

test('proxy page only exposes proxy-chain modes and does not render a page-level direct button', () => {
  const source = readFileSync(proxiesPagePath, 'utf8')

  assert.match(source, /const PROXY_CHAIN_MODES = \['rule', 'global'\] as const/)
  assert.match(source, /const PROXY_CHAIN_MODE_LABELS = \{[\s\S]*?rule: '应用规则'[\s\S]*?global: '不应用规则'[\s\S]*?\}/)
  assert.match(source, /PROXY_CHAIN_MODES\.map\(\(mode\)/)
  assert.doesNotMatch(source, /CLASH_MODES/)
  assert.doesNotMatch(source, /proxies\.page\.modes\.\$\{mode\}/)
  assert.doesNotMatch(source, /t\(mode === 'rule' \? 'proxies\.page\.modes\.rule' : 'proxies\.page\.modes\.global'\)/)
})

test('proxy groups normalize direct mode to rule for proxy-chain display', () => {
  const source = readFileSync(proxyGroupsPath, 'utf8')

  assert.match(source, /const displayMode = mode === 'direct' \? 'rule' : mode/)
  assert.match(source, /useChainMode\(\{[\s\S]*mode: displayMode/)
  assert.match(source, /useProxyGroups\(\{[\s\S]*mode: displayMode/)
  assert.match(source, /useScrollPosition\(\{[\s\S]*mode: displayMode/)
})
