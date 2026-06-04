import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const selectPath = join(repoRoot, 'src', 'components', 'tailwind', 'Select.tsx')

test('native select dropdown options keep dark theme contrast', () => {
  const source = readFileSync(selectPath, 'utf8')

  assert.match(source, /const nativeOptionClasses =/)
  assert.match(source, /\[color-scheme:dark\]/)
  assert.match(source, /\[&>option\]:bg-\[var\(--color-card\)\]/)
  assert.match(source, /\[&>option\]:text-\[var\(--color-text-primary\)\]/)
  assert.match(source, /\[&>option:checked\]:bg-\[rgba\(0,255,65,0\.18\)\]/)
})
