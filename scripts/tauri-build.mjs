import { spawn } from 'child_process'
import fs from 'fs'
import os from 'os'
import path from 'path'

const env = { ...process.env }
const args = process.argv.slice(2)

function resolveLocalSigningKeyPath() {
  return path.join(
    os.homedir(),
    '.tauri',
    'clash-verge-optimized',
    'updater.key',
  )
}

function injectLocalSigningKey() {
  if (!env.TAURI_SIGNING_PRIVATE_KEY) {
    const keyPath = resolveLocalSigningKeyPath()

    if (fs.existsSync(keyPath)) {
      env.TAURI_SIGNING_PRIVATE_KEY = fs.readFileSync(keyPath, 'utf8').trim()
      console.log(`[INFO]: Loaded TAURI_SIGNING_PRIVATE_KEY from ${keyPath}`)
    }
  }
}

function run() {
  injectLocalSigningKey()

  const command = 'pnpm'
  const child = spawn(command, ['exec', 'tauri', 'build', ...args], {
    stdio: 'inherit',
    env,
    shell: process.platform === 'win32',
  })

  child.on('error', (error) => {
    console.error('[ERROR]: Failed to start tauri build:', error)
    process.exit(1)
  })

  child.on('exit', (code) => {
    process.exit(code ?? 1)
  })
}

run()
