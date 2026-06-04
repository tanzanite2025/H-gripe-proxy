import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { dirname, join, normalize } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const fontStylePath = join(repoRoot, 'src', 'assets', 'styles', 'font.scss')

test('font face asset urls resolve to checked-in files', () => {
  const source = readFileSync(fontStylePath, 'utf8')
  const urls = [...source.matchAll(/url\(['"]?([^'")]+)['"]?\)/g)].map(
    ([, url]) => url,
  )

  assert.ok(urls.length > 0, 'font.scss should declare local font assets')
  for (const url of urls) {
    assert.ok(
      existsSync(join(dirname(fontStylePath), normalize(url))),
      `${url} should exist on disk`,
    )
  }
})
