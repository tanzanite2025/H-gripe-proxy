import { execFileSync, spawnSync } from 'child_process'
import { createHash } from 'crypto'
import fs from 'fs'
import fsp from 'fs/promises'
import path from 'path'

const cwd = process.cwd()
const MIHOMO_SOURCE_DIR = path.join(cwd, 'mihomo')
const SIDECAR_DIR = path.join(cwd, 'src-tauri', 'sidecar')
const MODULE_PATH = 'github.com/tanzanite2025/mihomo-optimized'

const TARGETS = {
  'x86_64-pc-windows-msvc': {
    goos: 'windows',
    goarch: 'amd64',
    goamd64: 'v2',
    exe: true,
  },
  'i686-pc-windows-msvc': { goos: 'windows', goarch: '386', exe: true },
  'aarch64-pc-windows-msvc': { goos: 'windows', goarch: 'arm64', exe: true },
  'x86_64-apple-darwin': { goos: 'darwin', goarch: 'amd64' },
  'aarch64-apple-darwin': { goos: 'darwin', goarch: 'arm64' },
  'x86_64-unknown-linux-gnu': {
    goos: 'linux',
    goarch: 'amd64',
    goamd64: 'v2',
  },
  'i686-unknown-linux-gnu': { goos: 'linux', goarch: '386' },
  'aarch64-unknown-linux-gnu': { goos: 'linux', goarch: 'arm64' },
  'armv7-unknown-linux-gnueabihf': {
    goos: 'linux',
    goarch: 'arm',
    goarm: '7',
  },
  'riscv64gc-unknown-linux-gnu': { goos: 'linux', goarch: 'riscv64' },
  'loongarch64-unknown-linux-gnu': { goos: 'linux', goarch: 'loong64' },
}

function readArgValue(name) {
  const index = process.argv.indexOf(name)
  if (index === -1) return null
  return process.argv[index + 1] ?? null
}

function currentRustTarget() {
  const output = execFileSync('rustc', ['-vV'], { encoding: 'utf-8' })
  return output.match(/(?<=host: ).+/)?.[0]?.trim()
}

function resolveTarget() {
  const explicit = readArgValue('--target')
  const positional = process.argv
    .slice(2)
    .find((arg) => !arg.startsWith('-') && !arg.includes('='))
  const target = explicit || positional || currentRustTarget()
  const config = TARGETS[target]
  if (!config) throw new Error(`unsupported mihomo sidecar target: ${target}`)
  return { target, config }
}

function shouldIncludeSourceFile(name) {
  return (
    name.endsWith('.go') ||
    name === 'go.mod' ||
    name === 'go.sum' ||
    name === 'Makefile'
  )
}

async function collectSourceFiles(dir) {
  const files = []

  async function walk(currentDir) {
    const entries = await fsp.readdir(currentDir, { withFileTypes: true })
    for (const entry of entries) {
      if (['.git', 'bin', 'dist', 'vendor'].includes(entry.name)) continue

      const entryPath = path.join(currentDir, entry.name)
      if (entry.isDirectory()) {
        await walk(entryPath)
      } else if (shouldIncludeSourceFile(entry.name)) {
        files.push(entryPath)
      }
    }
  }

  await walk(dir)
  return files.sort()
}

async function calculateSourceTreeHash() {
  const hash = createHash('sha256')
  for (const file of await collectSourceFiles(MIHOMO_SOURCE_DIR)) {
    const relative = path.relative(MIHOMO_SOURCE_DIR, file).replace(/\\/g, '/')
    hash.update(relative)
    hash.update('\0')
    hash.update(await fsp.readFile(file))
    hash.update('\0')
  }
  return hash.digest('hex')
}

async function calculateFileHash(filePath) {
  const hash = createHash('sha256')
  hash.update(await fsp.readFile(filePath))
  return hash.digest('hex')
}

function git(args, options = {}) {
  return execFileSync('git', args, {
    cwd: options.cwd ?? cwd,
    encoding: 'utf-8',
  }).trim()
}

function sourceCommit() {
  return git(['rev-parse', 'HEAD'], { cwd: MIHOMO_SOURCE_DIR })
}

function sourceDirty() {
  return (
    git(['status', '--porcelain', '--', '.'], {
      cwd: MIHOMO_SOURCE_DIR,
    }).length > 0
  )
}

async function build() {
  if (!fs.existsSync(MIHOMO_SOURCE_DIR)) {
    throw new Error(`missing mihomo source directory: ${MIHOMO_SOURCE_DIR}`)
  }

  const { target, config } = resolveTarget()
  const version =
    readArgValue('--version') ||
    process.env.MIHOMO_VERSION ||
    `local-${sourceCommit().slice(0, 12)}`
  const buildTime =
    process.env.SOURCE_DATE_EPOCH != null
      ? new Date(Number(process.env.SOURCE_DATE_EPOCH) * 1000).toISOString()
      : new Date().toISOString()
  const targetFile = `verge-mihomo-${target}${config.exe ? '.exe' : ''}`
  const sidecarPath = path.join(SIDECAR_DIR, targetFile)

  await fsp.mkdir(SIDECAR_DIR, { recursive: true })

  const env = {
    ...process.env,
    CGO_ENABLED: '0',
    GOOS: config.goos,
    GOARCH: config.goarch,
  }
  if (config.goamd64) env.GOAMD64 = config.goamd64
  if (config.goarm) env.GOARM = config.goarm

  const ldflags = [
    `-X ${MODULE_PATH}/constant.Version=${version}`,
    `-X ${MODULE_PATH}/constant.BuildTime=${buildTime}`,
    '-w',
    '-s',
    '-buildid=',
  ].join(' ')

  const result = spawnSync(
    'go',
    [
      'build',
      '-v',
      '-tags',
      'with_gvisor',
      '-trimpath',
      '-ldflags',
      ldflags,
      '-o',
      sidecarPath,
    ],
    {
      cwd: MIHOMO_SOURCE_DIR,
      env,
      stdio: 'inherit',
    },
  )

  if (result.error) throw result.error
  if (result.status !== 0) {
    throw new Error(`go build failed with exit code ${result.status}`)
  }

  if (!config.exe) await fsp.chmod(sidecarPath, 0o755)

  const metadata = {
    schemaVersion: 1,
    core: 'verge-mihomo',
    module: MODULE_PATH,
    target,
    goos: config.goos,
    goarch: config.goarch,
    goamd64: config.goamd64 ?? null,
    goarm: config.goarm ?? null,
    version,
    buildTime,
    sourceCommit: sourceCommit(),
    sourceDirty: sourceDirty(),
    sourceTreeHash: await calculateSourceTreeHash(),
    binarySha256: await calculateFileHash(sidecarPath),
    goVersion: execFileSync('go', ['version'], { encoding: 'utf-8' }).trim(),
  }

  await fsp.writeFile(
    `${sidecarPath}.build.json`,
    `${JSON.stringify(metadata, null, 2)}\n`,
  )
  console.log(`[INFO]: built ${sidecarPath}`)
  console.log(`[INFO]: wrote ${sidecarPath}.build.json`)
}

build().catch((error) => {
  console.error('[ERROR]: build-mihomo-sidecar failed:', error.message)
  process.exit(1)
})
