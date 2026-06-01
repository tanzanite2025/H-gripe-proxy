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
  const frontend = read('src/services/cmds.ts')
  const commandModule = read('src-tauri/src/cmd/clash.rs')
  const registry = read('src-tauri/src/lib.rs')

  assert.match(frontend, /invoke<void>\('clear_logs'\)/)
  assert.match(commandModule, /#\[tauri::command\][\s\S]*pub async fn clear_logs\(/)
  assert.match(registry, /cmd::clear_logs/)
})

test('proxy delay helper uses the registered mihomo plugin command', () => {
  const frontend = read('src/services/cmds.ts')
  const plugin = read('crates/tauri-plugin-mihomo/src/lib.rs')

  assert.doesNotMatch(frontend, /clash_api_get_proxy_delay/)
  assert.match(frontend, /invoke<\{\s*delay: number\s*\}>\(\s*'plugin:mihomo\|delay_proxy_by_name'/)
  assert.match(frontend, /proxyName:\s*name/)
  assert.match(frontend, /\btestUrl\b/)
  assert.match(plugin, /commands::delay_proxy_by_name/)
})
