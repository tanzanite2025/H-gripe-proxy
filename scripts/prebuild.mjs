import { execFileSync, execSync } from 'child_process'
import { createHash } from 'crypto'
import fs from 'fs'
import fsp from 'fs/promises'
import path from 'path'

import { glob } from 'glob'

import { log_debug, log_error, log_success } from './utils.mjs'

const cwd = process.cwd()
const TEMP_DIR = path.join(cwd, 'node_modules/.verge')
const FORCE = process.argv.includes('--force') || process.argv.includes('-f')
const HASH_CACHE_FILE = path.join(TEMP_DIR, '.hash_cache.json')
const MIHOMO_SOURCE_DIR = path.join(cwd, 'mihomo')
const SIDECAR_METADATA_SUFFIX = '.build.json'
const RESOURCES_DIR = path.join(cwd, 'src-tauri', 'resources')
const SIDECAR_DIR = path.join(cwd, 'src-tauri', 'sidecar')
const SERVICE_CRATE_DIR = path.join(cwd, 'crates', 'clash-verge-service-ipc')
const SIMPLE_SC_SOURCE = path.join(
  cwd,
  'src-tauri',
  'packages',
  'windows',
  'SimpleSC.dll',
)

const PLATFORM_MAP = {
  'x86_64-pc-windows-msvc': 'win32',
  'i686-pc-windows-msvc': 'win32',
  'aarch64-pc-windows-msvc': 'win32',
  'x86_64-apple-darwin': 'darwin',
  'aarch64-apple-darwin': 'darwin',
  'x86_64-unknown-linux-gnu': 'linux',
  'i686-unknown-linux-gnu': 'linux',
  'aarch64-unknown-linux-gnu': 'linux',
  'armv7-unknown-linux-gnueabihf': 'linux',
  'riscv64gc-unknown-linux-gnu': 'linux',
  'loongarch64-unknown-linux-gnu': 'linux',
}

const ARCH_MAP = {
  'x86_64-pc-windows-msvc': 'x64',
  'i686-pc-windows-msvc': 'ia32',
  'aarch64-pc-windows-msvc': 'arm64',
  'x86_64-apple-darwin': 'x64',
  'aarch64-apple-darwin': 'arm64',
  'x86_64-unknown-linux-gnu': 'x64',
  'i686-unknown-linux-gnu': 'ia32',
  'aarch64-unknown-linux-gnu': 'arm64',
  'armv7-unknown-linux-gnueabihf': 'arm',
  'riscv64gc-unknown-linux-gnu': 'riscv64',
  'loongarch64-unknown-linux-gnu': 'loong64',
}

const arg1 = process.argv.slice(2)[0]
const arg2 = process.argv.slice(2)[1]
const target = arg1 === '--force' || arg1 === '-f' ? arg2 : arg1
const { platform, arch } = target
  ? { platform: PLATFORM_MAP[target], arch: ARCH_MAP[target] }
  : process

const SIDECAR_HOST = target
  ? target
  : execSync('rustc -vV')
      .toString()
      .match(/(?<=host: ).+(?=\s*)/g)[0]

const SERVICE_DIR = platform === 'linux' ? SIDECAR_DIR : RESOURCES_DIR

const LOCAL_RESOURCE_SOURCES = {
  'Country.mmdb': path.join(RESOURCES_DIR, 'Country.mmdb'),
  'geosite.dat': path.join(RESOURCES_DIR, 'geosite.dat'),
  'geoip.dat': path.join(RESOURCES_DIR, 'geoip.dat'),
  'enableLoopback.exe': path.join(RESOURCES_DIR, 'enableLoopback.exe'),
}

const SERVICE_BINARIES = [
  'clash-verge-service',
  'clash-verge-service-install',
  'clash-verge-service-uninstall',
]

const META_MAP = {
  'win32-x64': 'mihomo-windows-amd64-v2',
  'win32-ia32': 'mihomo-windows-386',
  'win32-arm64': 'mihomo-windows-arm64',
  'darwin-x64': 'mihomo-darwin-amd64-v2-go122',
  'darwin-arm64': 'mihomo-darwin-arm64-go122',
  'linux-x64': 'mihomo-linux-amd64-v2',
  'linux-ia32': 'mihomo-linux-386',
  'linux-arm64': 'mihomo-linux-arm64',
  'linux-arm': 'mihomo-linux-armv7',
  'linux-riscv64': 'mihomo-linux-riscv64',
  'linux-loong64': 'mihomo-linux-loong64',
}

if (!META_MAP[`${platform}-${arch}`]) {
  throw new Error(`verge-mihomo unsupported platform "${platform}-${arch}"`)
}

async function calculateFileHash(filePath) {
  try {
    const fileBuffer = await fsp.readFile(filePath)
    const hashSum = createHash('sha256')
    hashSum.update(fileBuffer)
    return hashSum.digest('hex')
  } catch {
    return null
  }
}

async function loadHashCache() {
  try {
    if (fs.existsSync(HASH_CACHE_FILE)) {
      const data = await fsp.readFile(HASH_CACHE_FILE, 'utf-8')
      return JSON.parse(data)
    }
  } catch (err) {
    log_debug('Failed to load hash cache:', err.message)
  }
  return {}
}

async function saveHashCache(cache) {
  try {
    await fsp.mkdir(TEMP_DIR, { recursive: true })
    await fsp.writeFile(HASH_CACHE_FILE, JSON.stringify(cache, null, 2))
    log_debug('Hash cache saved')
  } catch (err) {
    log_debug('Failed to save hash cache:', err.message)
  }
}

async function hasFileChanged(filePath, targetPath) {
  if (FORCE) return true
  if (!fs.existsSync(targetPath)) return true
  const hashCache = await loadHashCache()
  const sourceHash = await calculateFileHash(filePath)
  const targetHash = await calculateFileHash(targetPath)
  if (!sourceHash || !targetHash) return true
  const cachedHash = hashCache[targetPath]
  return !(cachedHash === sourceHash && sourceHash === targetHash)
}

async function updateHashCache(targetPath) {
  const hashCache = await loadHashCache()
  const hash = await calculateFileHash(targetPath)
  if (hash) {
    hashCache[targetPath] = hash
    await saveHashCache(hashCache)
  }
}

function shouldIncludeMihomoSourceFile(name) {
  return (
    name.endsWith('.go') ||
    name === 'go.mod' ||
    name === 'go.sum' ||
    name === 'Makefile'
  )
}

async function collectMihomoSourceFiles(dir) {
  const files = []

  async function walk(currentDir) {
    let entries
    try {
      entries = await fsp.readdir(currentDir, { withFileTypes: true })
    } catch {
      return
    }

    for (const entry of entries) {
      if (['.git', 'bin', 'dist', 'vendor'].includes(entry.name)) continue

      const entryPath = path.join(currentDir, entry.name)
      if (entry.isDirectory()) {
        await walk(entryPath)
      } else if (shouldIncludeMihomoSourceFile(entry.name)) {
        files.push(entryPath)
      }
    }
  }

  await walk(dir)
  return files.sort()
}

async function calculateMihomoSourceTreeHash() {
  const hash = createHash('sha256')
  for (const file of await collectMihomoSourceFiles(MIHOMO_SOURCE_DIR)) {
    const relative = path.relative(MIHOMO_SOURCE_DIR, file).replace(/\\/g, '/')
    hash.update(relative)
    hash.update('\0')
    hash.update(await fsp.readFile(file))
    hash.update('\0')
  }
  return hash.digest('hex')
}

async function readSidecarMetadata(sidecarPath) {
  const metadataPath = `${sidecarPath}${SIDECAR_METADATA_SUFFIX}`
  try {
    return JSON.parse(await fsp.readFile(metadataPath, 'utf-8'))
  } catch (err) {
    throw new Error(
      [
        `Missing or invalid sidecar metadata "${metadataPath}".`,
        'Run `pnpm mihomo:sidecar -- --target <rust-target>` to rebuild the local core sidecar before packaging.',
        err.message,
      ].join(' '),
      { cause: err },
    )
  }
}

async function assertLocalSidecarMatchesSource(sidecarPath) {
  if (!fs.existsSync(MIHOMO_SOURCE_DIR)) return

  const metadata = await readSidecarMetadata(sidecarPath)
  const [sourceTreeHash, binarySha256] = await Promise.all([
    calculateMihomoSourceTreeHash(),
    calculateFileHash(sidecarPath),
  ])

  if (metadata.sourceTreeHash !== sourceTreeHash) {
    throw new Error(
      [
        `Local sidecar source hash mismatch: "${sidecarPath}".`,
        `metadata=${metadata.sourceTreeHash ?? '<missing>'}`,
        `current=${sourceTreeHash}`,
        'Rebuild mihomo with `pnpm mihomo:sidecar -- --target <rust-target>` before packaging.',
      ].join(' '),
    )
  }

  if (metadata.binarySha256 !== binarySha256) {
    throw new Error(
      [
        `Local sidecar binary hash mismatch: "${sidecarPath}".`,
        `metadata=${metadata.binarySha256 ?? '<missing>'}`,
        `current=${binarySha256}`,
      ].join(' '),
    )
  }
}

function clashMeta() {
  const isWin = platform === 'win32'
  return {
    name: 'verge-mihomo',
    targetFile: `verge-mihomo-${SIDECAR_HOST}${isWin ? '.exe' : ''}`,
  }
}

async function resolveLocalSidecar(binInfo) {
  const { name, targetFile } = binInfo
  const sidecarPath = path.join(SIDECAR_DIR, targetFile)

  await fsp.mkdir(SIDECAR_DIR, { recursive: true })

  if (!fs.existsSync(sidecarPath)) {
    throw new Error(
      `Missing local sidecar "${sidecarPath}". Please place your locally managed ${name} binary there before running prebuild.`,
    )
  }

  if (platform !== 'win32') {
    await fsp.chmod(sidecarPath, 0o755)
  }

  await assertLocalSidecarMatchesSource(sidecarPath)
  log_success(`Using local sidecar: "${sidecarPath}"`)
}

async function resolveLocalResource(file, localPath, options = {}) {
  const { dir = RESOURCES_DIR, requiredOn = () => true } = options
  if (!requiredOn()) return

  const targetPath = path.join(dir, file)
  const sourcePath = path.resolve(localPath)

  if (!fs.existsSync(sourcePath)) {
    throw new Error(
      `Missing local controlled resource "${sourcePath}" for "${file}". Prebuild no longer downloads this asset automatically.`,
    )
  }

  if (sourcePath === targetPath) {
    await updateHashCache(targetPath)
    log_success(`Validated local controlled resource: "${file}"`)
    return
  }

  if (!(await hasFileChanged(sourcePath, targetPath))) {
    return
  }

  await fsp.mkdir(dir, { recursive: true })
  await fsp.copyFile(sourcePath, targetPath)
  if (platform !== 'win32' && /\.(sh|bin|exe)?$/i.test(file)) {
    try {
      await fsp.chmod(targetPath, 0o755)
    } catch (err) {
      log_debug(`chmod skipped for "${targetPath}":`, err.message)
    }
  }
  await updateHashCache(targetPath)
  log_success(`Copied local controlled resource: "${file}"`)
}

function serviceFileInfo(name) {
  const ext = platform === 'win32' ? '.exe' : ''
  const suffix = platform === 'linux' ? `-${SIDECAR_HOST}` : ''
  return {
    sourceFile: `${name}${ext}`,
    targetFile: `${name}${suffix}${ext}`,
  }
}

function cargoBinaryName(name) {
  return `${name}${platform === 'win32' ? '.exe' : ''}`
}

function buildServiceBundle() {
  const bins = SERVICE_BINARIES.flatMap((name) => ['--bin', name])
  const args = [
    'build',
    '--release',
    '--manifest-path',
    path.join(SERVICE_CRATE_DIR, 'Cargo.toml'),
    '--features',
    'standalone',
  ]
  if (target) {
    args.push('--target', target)
  }
  args.push(...bins)

  execFileSync('cargo', args, {
    cwd,
    env: process.env,
    stdio: 'inherit',
  })
}

function builtServiceBinaryPath(name) {
  const profileDir = target
    ? path.join(cwd, 'target', target, 'release')
    : path.join(cwd, 'target', 'release')
  return path.join(profileDir, cargoBinaryName(name))
}

async function resolveServiceBundle() {
  const files = SERVICE_BINARIES.map((name) => {
    const info = serviceFileInfo(name)
    return {
      name,
      sourcePath: builtServiceBinaryPath(name),
      targetPath: path.join(SERVICE_DIR, info.targetFile),
    }
  })

  const needsBuild =
    FORCE || files.some(({ targetPath }) => !fs.existsSync(targetPath))

  if (needsBuild) {
    buildServiceBundle()
  }

  await fsp.mkdir(SERVICE_DIR, { recursive: true })

  for (const { name, sourcePath, targetPath } of files) {
    if (!fs.existsSync(sourcePath)) {
      throw new Error(
        `Missing locally built service binary "${sourcePath}" for "${name}".`,
      )
    }

    if (!(await hasFileChanged(sourcePath, targetPath))) {
      continue
    }

    await fsp.copyFile(sourcePath, targetPath)
    if (platform !== 'win32') await fsp.chmod(targetPath, 0o755)
    await updateHashCache(targetPath)
    log_success(`Installed local service binary: "${path.basename(targetPath)}"`)
  }
}

const resolvePlugin = async () => {
  const pluginDir = path.join(process.env.APPDATA || '', 'Local/NSIS')
  const pluginPath = path.join(pluginDir, 'SimpleSC.dll')

  if (!fs.existsSync(SIMPLE_SC_SOURCE)) {
    throw new Error(
      `Missing local controlled resource "${SIMPLE_SC_SOURCE}" for "SimpleSC.dll".`,
    )
  }

  if (!(await hasFileChanged(SIMPLE_SC_SOURCE, pluginPath))) {
    return
  }

  await fsp.mkdir(pluginDir, { recursive: true })
  await fsp.copyFile(SIMPLE_SC_SOURCE, pluginPath)
  await updateHashCache(pluginPath)
  log_success('Installed local NSIS SimpleSC.dll')
}

const resolveServicePermission = async () => {
  const serviceExecutables = [
    'clash-verge-service*',
    'clash-verge-service-install*',
    'clash-verge-service-uninstall*',
  ]
  const hashCache = await loadHashCache()
  let hasChanges = false

  for (const pattern of serviceExecutables) {
    const files = glob.sync(path.join(SERVICE_DIR, pattern))
    for (const filePath of files) {
      if (!fs.existsSync(filePath)) continue
      const currentHash = await calculateFileHash(filePath)
      const cacheKey = `${filePath}_chmod`
      if (!FORCE && hashCache[cacheKey] === currentHash) {
        continue
      }
      try {
        await fsp.chmod(filePath, 0o755)
        log_success(`chmod finished: "${filePath}"`)
      } catch (err) {
        log_error(`chmod failed for ${filePath}:`, err.message)
      }
      hashCache[cacheKey] = currentHash
      hasChanges = true
    }
  }

  if (hasChanges) {
    await saveHashCache(hashCache)
  }
}

const resolveMmdb = () =>
  resolveLocalResource('Country.mmdb', LOCAL_RESOURCE_SOURCES['Country.mmdb'])

const resolveGeosite = () =>
  resolveLocalResource('geosite.dat', LOCAL_RESOURCE_SOURCES['geosite.dat'])

const resolveGeoIP = () =>
  resolveLocalResource('geoip.dat', LOCAL_RESOURCE_SOURCES['geoip.dat'])

const resolveEnableLoopback = () =>
  resolveLocalResource(
    'enableLoopback.exe',
    LOCAL_RESOURCE_SOURCES['enableLoopback.exe'],
    { requiredOn: () => platform === 'win32' },
  )

const resolveSetDnsScript = () =>
  resolveLocalResource('set_dns.sh', path.join(cwd, 'scripts', 'set_dns.sh'), {
    requiredOn: () => platform === 'darwin',
  })

const resolveUnSetDnsScript = () =>
  resolveLocalResource(
    'unset_dns.sh',
    path.join(cwd, 'scripts', 'unset_dns.sh'),
    { requiredOn: () => platform === 'darwin' },
  )

const tasks = [
  {
    name: 'verge-mihomo',
    func: () => resolveLocalSidecar(clashMeta()),
    retry: 1,
  },
  { name: 'plugin', func: resolvePlugin, retry: 1, winOnly: true },
  { name: 'service', func: resolveServiceBundle, retry: 1 },
  { name: 'mmdb', func: resolveMmdb, retry: 1 },
  { name: 'geosite', func: resolveGeosite, retry: 1 },
  { name: 'geoip', func: resolveGeoIP, retry: 1 },
  {
    name: 'enableLoopback',
    func: resolveEnableLoopback,
    retry: 1,
    winOnly: true,
  },
  {
    name: 'service_chmod',
    func: resolveServicePermission,
    retry: 1,
    unixOnly: platform === 'linux' || platform === 'darwin',
  },
  {
    name: 'set_dns_script',
    func: resolveSetDnsScript,
    retry: 1,
    macosOnly: true,
  },
  {
    name: 'unset_dns_script',
    func: resolveUnSetDnsScript,
    retry: 1,
    macosOnly: true,
  },
]

async function runTask() {
  const task = tasks.shift()
  if (!task) return
  if (task.unixOnly && platform === 'win32') return runTask()
  if (task.winOnly && platform !== 'win32') return runTask()
  if (task.macosOnly && platform !== 'darwin') return runTask()
  if (task.linuxOnly && platform !== 'linux') return runTask()

  for (let i = 0; i < task.retry; i++) {
    try {
      await task.func()
      break
    } catch (err) {
      log_error(`task::${task.name} try ${i} ==`, err.message)
      if (i === task.retry - 1) throw err
    }
  }
  return runTask()
}

runTask()
