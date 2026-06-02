import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const tauriConfigPath = join(repoRoot, 'src-tauri', 'tauri.conf.json')

test('tauri CSP allows internal IPC and asset hosts required by Tauri v2', () => {
  const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf8'))
  const csp = tauriConfig?.app?.security?.csp

  assert.equal(typeof csp, 'string', 'CSP should be configured as a string')
  assert.match(csp, /connect-src/, 'CSP should define connect-src')
  assert.match(csp, /http:\/\/ipc\.localhost/, 'connect-src should allow Tauri IPC host')
  assert.match(csp, /http:\/\/tauri\.localhost/, 'connect-src should allow Tauri asset host')
})
