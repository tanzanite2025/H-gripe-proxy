import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const proxyDetectionCardPath = join(
  repoRoot,
  'src',
  'components',
  'home',
  'proxy-detection-card.tsx',
)
const zhHomeLocalePath = join(repoRoot, 'src', 'locales', 'zh', 'home.json')
const enHomeLocalePath = join(repoRoot, 'src', 'locales', 'en', 'home.json')

test('proxy detection card avoids direct as a visible global-mode label', () => {
  const source = readFileSync(proxyDetectionCardPath, 'utf8')

  assert.doesNotMatch(source, /proxyDetection\.labels\.direct/)
  assert.match(source, /proxyDetection\.labels\.localEgress/)
  assert.match(source, /proxyDetection\.labels\.proxyEgress/)
})

test('proxy detection copy names egress paths instead of another direct mode', () => {
  const zh = JSON.parse(readFileSync(zhHomeLocalePath, 'utf8'))
  const en = JSON.parse(readFileSync(enHomeLocalePath, 'utf8'))
  const zhProxyDetection = zh.components.proxyDetection
  const enProxyDetection = en.components.proxyDetection

  assert.equal(
    zhProxyDetection.labels.localEgress,
    '本机出口',
  )
  assert.equal(
    zhProxyDetection.labels.proxyEgress,
    '代理链路出口',
  )
  assert.equal(
    enProxyDetection.labels.localEgress,
    'Local egress',
  )
  assert.equal(
    enProxyDetection.labels.proxyEgress,
    'Proxy-chain egress',
  )
  assert.equal('direct' in zhProxyDetection.labels, false)
  assert.equal('proxy' in zhProxyDetection.labels, false)
  assert.equal('direct' in enProxyDetection.labels, false)
  assert.equal('proxy' in enProxyDetection.labels, false)
  assert.doesNotMatch(JSON.stringify(zhProxyDetection), /直连/)
  assert.doesNotMatch(JSON.stringify(enProxyDetection), /\bDirect\b/)
})
