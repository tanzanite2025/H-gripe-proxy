import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const apiService = readFileSync(
  new URL('../src/services/api.ts', import.meta.url),
  'utf8',
)

const cmdService = readFileSync(
  new URL('../src/services/cmds.ts', import.meta.url),
  'utf8',
)

const runtimeCommands = readFileSync(
  new URL('../src-tauri/src/cmd/runtime.rs', import.meta.url),
  'utf8',
)

const tauriLib = readFileSync(
  new URL('../src-tauri/src/lib.rs', import.meta.url),
  'utf8',
)

test('IP info uses backend local-core observation before frontend direct fetch', () => {
  assert.match(cmdService, /get_current_public_ip_info/)
  assert.match(runtimeCommands, /pub async fn get_current_public_ip_info/)
  assert.match(tauriLib, /cmd::get_current_public_ip_info/)

  const backendCallIndex = apiService.indexOf('getCurrentPublicIpInfo')
  const directFetchIndex = apiService.indexOf('fetch(service.url')

  assert.notEqual(backendCallIndex, -1)
  assert.notEqual(directFetchIndex, -1)
  assert.ok(
    backendCallIndex < directFetchIndex,
    'backend public IP observation should be attempted before frontend direct IP services',
  )
})
