import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const read = (...segments) => readFileSync(join(repoRoot, ...segments), 'utf8')
const readJson = (...segments) => JSON.parse(read(...segments))

test('home system info running mode is not the TUN mode state', () => {
  const source = read('src', 'components', 'home', 'system-info-card.tsx')

  assert.match(source, /useSystemState\(\)/)
  assert.match(source, /getModeText/)
  assert.doesNotMatch(source, /enable_tun_mode/)
  assert.doesNotMatch(source, /tunMode/)
})

test('home running-mode copy describes runtime carrier instead of TUN mode', () => {
  const zh = readJson('src', 'locales', 'zh', 'home.json')
  const zhtw = readJson('src', 'locales', 'zhtw', 'home.json')
  const en = readJson('src', 'locales', 'en', 'home.json')

  assert.equal(zh.components.systemInfo.badges.serviceMode, '服务运行')
  assert.equal(zh.components.systemInfo.badges.sidecarMode, '用户态运行')
  assert.equal(
    zh.components.systemInfo.badges.adminServiceMode,
    '管理员 + 服务运行',
  )

  assert.equal(zhtw.components.systemInfo.badges.serviceMode, '服務執行')
  assert.equal(zhtw.components.systemInfo.badges.sidecarMode, '使用者態執行')
  assert.equal(
    zhtw.components.systemInfo.badges.adminServiceMode,
    '系統管理員 + 服務執行',
  )

  assert.equal(en.components.systemInfo.badges.serviceMode, 'Service Runtime')
  assert.equal(en.components.systemInfo.badges.sidecarMode, 'User Runtime')
  assert.equal(
    en.components.systemInfo.badges.adminServiceMode,
    'Admin + Service Runtime',
  )
})

test('Chinese TUN copy asks for service availability instead of service mode', () => {
  const zhHome = readJson('src', 'locales', 'zh', 'home.json')
  const zhSettings = readJson('src', 'locales', 'zh', 'settings.json')
  const zhtwHome = readJson('src', 'locales', 'zhtw', 'home.json')
  const zhtwSettings = readJson('src', 'locales', 'zhtw', 'settings.json')

  assert.doesNotMatch(zhHome.components.proxyTun.status.tunModeServiceRequired, /服务模式/)
  assert.doesNotMatch(
    zhSettings.sections.proxyControl.tooltips.tunUnavailable,
    /服务模式|管理员模式/,
  )
  assert.doesNotMatch(zhtwHome.components.proxyTun.status.tunModeServiceRequired, /服務模式/)
  assert.doesNotMatch(
    zhtwSettings.sections.proxyControl.tooltips.tunUnavailable,
    /服務模式|管理員模式/,
  )
})
