import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const currentProxyCardPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'current-proxy-card',
  'index.tsx',
)
const currentProxyDataPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'current-proxy-card',
  'hooks',
  'use-current-proxy-data.ts',
)
const proxyInfoDisplayPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'current-proxy-card',
  'components',
  'proxy-info-display.tsx',
)
const staleProxySelectorsPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'current-proxy-card',
  'components',
  'proxy-selectors.tsx',
)
const proxyDelayCheckPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'current-proxy-card',
  'hooks',
  'use-proxy-delay-check.ts',
)

test('current proxy card treats direct as a global outbound intent, not a card mode', () => {
  const source = readFileSync(currentProxyCardPath, 'utf8')

  assert.match(source, /const proxyChainMode = mode === 'direct' \? DEFAULT_CLASH_MODE : mode/)
  assert.match(source, /const isGlobalMode = proxyChainMode === 'global'/)
  assert.doesNotMatch(source, /const isDirectMode/)
  assert.doesNotMatch(source, /disabled=\{isDirectMode/)
  assert.doesNotMatch(source, /isDirectMode/)
})

test('current proxy data does not synthesize DIRECT as a homepage proxy selection', () => {
  const source = readFileSync(currentProxyDataPath, 'utf8')

  assert.doesNotMatch(source, /isDirectMode/)
  assert.doesNotMatch(source, /newGroup = 'DIRECT'/)
  assert.doesNotMatch(source, /newProxy = 'DIRECT'/)
  assert.doesNotMatch(source, /\{ name: 'DIRECT' \}/)
})

test('current proxy display and delay checks no longer expose direct mode UI branches', () => {
  const displaySource = readFileSync(proxyInfoDisplayPath, 'utf8')
  const delaySource = readFileSync(proxyDelayCheckPath, 'utf8')

  assert.doesNotMatch(displaySource, /isDirectMode/)
  assert.doesNotMatch(displaySource, /directMode/)
  assert.doesNotMatch(delaySource, /isDirectMode/)
})

test('stale current-proxy selector component is not kept as a second direct-mode branch', () => {
  assert.equal(existsSync(staleProxySelectorsPath), false)
})
