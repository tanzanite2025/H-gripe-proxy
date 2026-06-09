import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const clashModeServicePath = join(repoRoot, 'src', 'services', 'clash-mode.ts')
const backendClashModePath = join(repoRoot, 'src-tauri', 'src', 'core', 'clash_mode.rs')
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

test('current proxy card only derives global-vs-rule state from the public clash mode', () => {
  const source = readFileSync(currentProxyControllerPath, 'utf8')

  assert.match(source, /const mode =[\s\S]*?DEFAULT_CLASH_MODE/)
  assert.match(source, /const isGlobalMode = mode === 'global'/)
  assert.doesNotMatch(source, /proxyChainMode/)
  assert.doesNotMatch(source, /mode === 'direct'/)
  assert.doesNotMatch(source, /const isDirectMode/)
})

test('current proxy data does not synthesize DIRECT as a homepage proxy selection', () => {
  const source = readFileSync(currentProxyDataPath, 'utf8')

  assert.doesNotMatch(source, /isDirectMode/)
  assert.doesNotMatch(source, /newGroup = 'DIRECT'/)
  assert.doesNotMatch(source, /newProxy = 'DIRECT'/)
  assert.doesNotMatch(source, /\{ name: 'DIRECT' \}/)
})

test('current proxy display and delay checks do not expose direct-mode UI branches', () => {
  const displaySource = readFileSync(proxyInfoDisplayPath, 'utf8')
  const delaySource = readFileSync(proxyDelayCheckPath, 'utf8')

  assert.doesNotMatch(displaySource, /isDirectMode/)
  assert.doesNotMatch(displaySource, /directMode/)
  assert.doesNotMatch(delaySource, /isDirectMode/)
})

test('stale current-proxy selector component is not kept as a second mode branch', () => {
  assert.equal(existsSync(staleProxySelectorsPath), false)
})

test('clash mode direct is removed instead of being aliased anywhere', () => {
  const serviceSource = readFileSync(clashModeServicePath, 'utf8')
  const backendModeSource = readFileSync(backendClashModePath, 'utf8')

  assert.doesNotMatch(serviceSource, /LEGACY_CLASH_MODE_ALIASES/)
  assert.doesNotMatch(serviceSource, /direct:\s*DEFAULT_CLASH_MODE/)
  assert.doesNotMatch(backendModeSource, /"direct" => Ok\(Self::Rule\)/)
})
