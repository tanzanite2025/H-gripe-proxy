import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { test } from 'node:test'

const root = path.resolve(import.meta.dirname, '..')
const prebuild = fs.readFileSync(
  path.join(root, 'scripts/prebuild.mjs'),
  'utf8',
)

test('prebuild verifies local mihomo sidecar metadata', () => {
  assert.match(
    prebuild,
    /MIHOMO_SOURCE_DIR\s*=\s*path\.join\(cwd,\s*'mihomo'\)/,
    'prebuild should know where the local mihomo source lives',
  )
  assert.match(
    prebuild,
    /assertLocalSidecarMatchesSource\(sidecarPath\)/,
    'prebuild should verify source and binary hashes before accepting the sidecar',
  )
  assert.match(
    prebuild,
    /Local sidecar source hash mismatch/,
    'stale sidecar failures should explain the source mismatch',
  )
  assert.match(
    prebuild,
    /Local sidecar binary hash mismatch/,
    'tampered sidecar failures should explain the binary mismatch',
  )
})
