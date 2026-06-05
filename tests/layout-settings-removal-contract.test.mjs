import assert from 'node:assert/strict'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const read = (...segments) => readFileSync(join(repoRoot, ...segments), 'utf8')

const layoutSettingFields = [
  'traffic_graph',
  'enable_memory_usage',
  'enable_group_icon',
  'pause_render_traffic_stats_on_blur',
  'menu_icon',
  'notice_position',
  'collapse_navbar',
  'tray_icon',
  'common_tray_icon',
  'sysproxy_tray_icon',
  'tun_tray_icon',
  'enable_tray_speed',
  'tray_proxy_groups_display_mode',
  'tray_inline_outbound_modes',
  'enable_hover_jump_navigator',
  'hover_jump_navigator_delay',
]

test('Verge basic settings no longer exposes layout settings', () => {
  const source = read('src', 'components', 'setting', 'setting-verge-basic.tsx')

  assert.doesNotMatch(source, /LayoutViewer/)
  assert.doesNotMatch(source, /layoutRef/)
  assert.doesNotMatch(source, /layoutSetting/)
  assert.equal(
    existsSync(
      join(
        repoRoot,
        'src',
        'components',
        'setting',
        'components',
        'misc',
        'layout-config.tsx',
      ),
    ),
    false,
  )
})

test('frontend no longer reads user-configurable layout setting fields', () => {
  for (const file of [
    ['src', 'components', 'home', 'enhanced-traffic-stats.tsx'],
    ['src', 'components', 'home', 'enhanced-canvas-traffic-graph', 'index.tsx'],
    ['src', 'components', 'layout', 'layout-traffic.tsx'],
    ['src', 'components', 'layout', 'layout-item.tsx'],
    ['src', 'components', 'proxy', 'proxy-render.tsx'],
    ['src', 'components', 'proxy', 'proxy-groups', 'index.tsx'],
    ['src', 'pages', '_layout', 'layout.tsx'],
  ]) {
    const source = read(...file)

    for (const field of layoutSettingFields) {
      assert.doesNotMatch(source, new RegExp(`verge\\?\\.${field}`), file.join('/'))
    }
  }
})

test('layout defaults are fixed in frontend surfaces', () => {
  const homeTraffic = read('src', 'components', 'home', 'enhanced-traffic-stats.tsx')
  const enhancedCanvasTraffic = read(
    'src',
    'components',
    'home',
    'enhanced-canvas-traffic-graph',
    'index.tsx',
  )
  const layoutTraffic = read('src', 'components', 'layout', 'layout-traffic.tsx')
  const layoutItem = read('src', 'components', 'layout', 'layout-item.tsx')
  const proxyRender = read('src', 'components', 'proxy', 'proxy-render.tsx')
  const proxyGroups = read('src', 'components', 'proxy', 'proxy-groups', 'index.tsx')
  const layout = read('src', 'pages', '_layout', 'layout.tsx')

  assert.match(homeTraffic, /const trafficGraph = true/)
  assert.match(enhancedCanvasTraffic, /const pauseRenderOnBlur = true/)
  assert.match(layoutTraffic, /const trafficGraph = true/)
  assert.match(layoutTraffic, /const displayMemory = true/)
  assert.match(layoutItem, /const navCollapsed = false/)
  assert.match(proxyRender, /const enable_group_icon = true/)
  assert.match(proxyGroups, /enableHoverJump=\{true\}/)
  assert.match(proxyGroups, /hoverDelay=\{DEFAULT_HOVER_DELAY\}/)
  assert.match(layout, /<NoticeManager \/>/)
})

test('backend no longer accepts layout setting patches', () => {
  for (const file of [
    ['src-tauri', 'src', 'config', 'verge.rs'],
    ['src-tauri', 'src', 'feat', 'config.rs'],
    ['src', 'types', 'global.d.ts'],
  ]) {
    const source = read(...file)

    for (const field of layoutSettingFields) {
      assert.doesNotMatch(source, new RegExp(field), file.join('/'))
    }
  }
})

test('tray layout behavior is fixed to defaults', () => {
  const source = read('src-tauri', 'src', 'core', 'tray', 'mod.rs')

  assert.doesNotMatch(
    source,
    /verge(?:_settings)?\.(?:tray_proxy_groups_display_mode|tray_inline_outbound_modes|enable_tray_speed|common_tray_icon|sysproxy_tray_icon|tun_tray_icon|tray_icon)/,
  )
  assert.match(source, /let tray_proxy_groups_display_mode = "default"/)
  assert.match(source, /let show_outbound_modes_inline = false/)
})

test('locale resources no longer include layout settings copy', () => {
  for (const localeName of readdirSync(join(repoRoot, 'src', 'locales'))) {
    const source = read('src', 'locales', localeName, 'settings.json')

    assert.doesNotMatch(source, /"layoutSetting"/, localeName)
    assert.doesNotMatch(source, /"layout"\s*:/, localeName)
  }
})

test('generated i18n types no longer include layout settings keys', () => {
  for (const file of [
    ['src', 'types', 'generated', 'i18n-keys.ts'],
    ['src', 'types', 'generated', 'i18n-resources.ts'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /settings\.components\.verge\.basic\.fields\.layoutSetting/)
    assert.doesNotMatch(source, /settings\.components\.verge\.layout/)
    assert.doesNotMatch(source, /layoutSetting:\s*string/)
    assert.doesNotMatch(source, /layout:\s*\{[\s\S]*proxyGroupIcon:\s*string/)
  }
})
