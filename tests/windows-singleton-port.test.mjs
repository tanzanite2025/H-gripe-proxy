import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { test } from 'node:test'

const root = path.resolve(import.meta.dirname, '..')
const constants = fs.readFileSync(
  path.join(root, 'src-tauri/src/constants.rs'),
  'utf8',
)

test('release singleton port is unique to this fork', () => {
  const releasePortMatch = constants.match(
    /#\[cfg\(not\(feature = "verge-dev"\)\)\]\s+pub const SINGLETON_SERVER: u16 = (\d+);/,
  )

  assert.ok(releasePortMatch, 'release singleton port should be configured')
  assert.equal(
    releasePortMatch[1],
    '33341',
    'release singleton port must not collide with Clash Verge/Party forks',
  )
  assert.notEqual(releasePortMatch[1], '33331')
})
