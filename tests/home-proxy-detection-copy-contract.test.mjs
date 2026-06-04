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

  assert.equal(
    zh.components.proxyDetection.labels.localEgress,
    '本机出口',
  )
  assert.equal(
    zh.components.proxyDetection.labels.proxyEgress,
    '代理链路出口',
  )
  assert.equal(
    en.components.proxyDetection.labels.localEgress,
    'Local egress',
  )
  assert.equal(
    en.components.proxyDetection.labels.proxyEgress,
    'Proxy-chain egress',
  )
  assert.notEqual(zh.components.proxyDetection.labels.localEgress, '直连')
  assert.notEqual(en.components.proxyDetection.labels.localEgress, 'Direct')
})
