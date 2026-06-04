import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const mainPath = join(repoRoot, 'src', 'main.tsx')

test('main entry only performs global websocket cleanup while unloading', () => {
  const source = readFileSync(mainPath, 'utf8')
  const cleanupCalls = source.match(/MihomoWebSocket\.cleanupAll\(\)/g) ?? []

  assert.equal(
    cleanupCalls.length,
    1,
    'global cleanup should not run from multiple lifecycle events',
  )
  assert.match(
    source,
    /addEventListener\('beforeunload'[\s\S]*MihomoWebSocket\.cleanupAll\(\)/,
    'cleanup should remain tied to page unload',
  )
  assert.doesNotMatch(
    source,
    /addEventListener\('DOMContentLoaded'[\s\S]*MihomoWebSocket\.cleanupAll\(\)/,
    'DOMContentLoaded must not close live websocket subscriptions',
  )
})
