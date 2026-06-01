import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { test } from 'node:test'

const root = path.resolve(import.meta.dirname, '..')
const prebuild = fs.readFileSync(path.join(root, 'scripts/prebuild.mjs'), 'utf8')

test('prebuild rejects stale local mihomo sidecar', () => {
  assert.match(
    prebuild,
    /MIHOMO_SOURCE_DIR\s*=\s*path\.join\(cwd,\s*'mihomo'\)/,
    'prebuild should know where the local mihomo source lives',
  )
  assert.match(
    prebuild,
    /assertLocalSidecarFresh\(sidecarPath\)/,
    'prebuild should verify source freshness before accepting the sidecar',
  )
  assert.match(
    prebuild,
    /Local sidecar is older than mihomo source/,
    'stale sidecar failures should explain the packaging risk',
  )
})
