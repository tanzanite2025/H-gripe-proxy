import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { test } from 'node:test'

const root = path.resolve(import.meta.dirname, '..')

const read = (file) => fs.readFileSync(path.join(root, file), 'utf8')

test('TLS fingerprint clear command is implemented and registered', () => {
  const frontend = read('src/services/tls-fingerprint.ts')
  const commandModule = read('src-tauri/src/cmd/tls_fingerprint.rs')
  const registry = read('src-tauri/src/lib.rs')

  assert.match(frontend, /invoke<void>\('tls_fingerprint_clear'\)/)
  assert.match(commandModule, /#\[tauri::command\][\s\S]*pub fn tls_fingerprint_clear\(/)
  assert.match(registry, /cmd::tls_fingerprint_clear/)
})

test('clear logs command is implemented and registered', () => {
  const frontend = read('src/services/cmds/runtime.ts')
  const commandModule = read('src-tauri/src/cmd/clash.rs')
  const registry = read('src-tauri/src/lib.rs')

  assert.match(frontend, /invoke<void>\('clear_logs'\)/)
  assert.match(commandModule, /#\[tauri::command\][\s\S]*pub async fn clear_logs\(/)
  assert.match(registry, /cmd::clear_logs/)
})

test('latency planning uses the Rust command path', () => {
  const runtimeCmds = read('src/services/cmds/runtime.ts')
  const delayManager = read('src/services/delay.ts')
  const commandModule = read('src-tauri/src/cmd/latency_test.rs')
  const registry = read('src-tauri/src/lib.rs')

  assert.doesNotMatch(runtimeCmds, /cmdGetProxyDelay/)
  assert.match(runtimeCmds, /invoke<LatencyTestPlan>\('plan_latency_test'/)
  assert.match(delayManager, /planLatencyTest\(\{/)
  assert.match(commandModule, /#\[tauri::command\][\s\S]*pub async fn plan_latency_test\(/)
  assert.match(registry, /cmd::plan_latency_test/)
})
