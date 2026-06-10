import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const clashModeServicePath = join(repoRoot, 'src', 'services', 'clash-mode.ts')
const currentProxyControllerPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'current-proxy-card',
  'hooks',
  'use-current-proxy-card-controller.ts',
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

test('current proxy card no longer depends on clash mode state', () => {
  const source = readFileSync(currentProxyControllerPath, 'utf8')

  assert.equal(existsSync(clashModeServicePath), false)
  assert.doesNotMatch(source, /DEFAULT_CLASH_MODE/)
  assert.doesNotMatch(source, /isGlobalMode/)
  assert.doesNotMatch(source, /proxyChainMode/)
  assert.doesNotMatch(source, /mode === 'global'/)
  assert.doesNotMatch(source, /mode === 'rule'/)
})

test('current proxy data does not synthesize DIRECT as a homepage proxy selection', () => {
  const source = readFileSync(currentProxyDataPath, 'utf8')

  assert.doesNotMatch(source, /isDirectMode/)
  assert.doesNotMatch(source, /newGroup = 'DIRECT'/)
  assert.doesNotMatch(source, /newProxy = 'DIRECT'/)
  assert.doesNotMatch(source, /\{ name: 'DIRECT' \}/)
})

test('current proxy display and delay checks do not expose stale mode branches', () => {
  const displaySource = readFileSync(proxyInfoDisplayPath, 'utf8')
  const delaySource = readFileSync(proxyDelayCheckPath, 'utf8')

  assert.doesNotMatch(displaySource, /isDirectMode|isGlobalMode|directMode|globalMode/)
  assert.doesNotMatch(delaySource, /isDirectMode|isGlobalMode|directMode|globalMode/)
})

test('stale current-proxy selector component is not kept as a second mode branch', () => {
  assert.equal(existsSync(staleProxySelectorsPath), false)
})
