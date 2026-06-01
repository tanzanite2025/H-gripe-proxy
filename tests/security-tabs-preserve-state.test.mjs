import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()

test('security tabs keep panel contents mounted while switching tabs', () => {
  const source = readFileSync(
    join(repoRoot, 'src', 'components', 'security', 'index.tsx'),
    'utf8',
  )

  assert.doesNotMatch(source, /\{value === index && <div>\{children\}<\/div>\}/)
  assert.match(source, /visitedTabs/)
  assert.match(source, /mounted=\{visitedTabs\.includes\(0\)\}/)
  assert.match(source, /mounted=\{visitedTabs\.includes\(3\)\}/)
  assert.match(source, /\{mounted && <div>\{children\}<\/div>\}/)
})
