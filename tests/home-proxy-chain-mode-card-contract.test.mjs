import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
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
