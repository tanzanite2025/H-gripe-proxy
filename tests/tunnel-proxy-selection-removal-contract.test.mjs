import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const read = (...segments) => readFileSync(join(repoRoot, ...segments), 'utf8')

test('tunnel settings no longer depend on proxy group data', () => {
  const source = read(
    'src',
    'components',
    'setting',
    'components',
    'network',
    'tunnels-config.tsx',
  )

  assert.doesNotMatch(source, /useProxiesData/)
  assert.doesNotMatch(source, /proxyGroups/)
  assert.doesNotMatch(source, /groupNames/)
  assert.doesNotMatch(source, /proxyOptions/)
  assert.doesNotMatch(source, /values\.group/)
})

test('new tunnel entries no longer write an explicit proxy override', () => {
  const source = read(
    'src',
    'components',
    'setting',
    'components',
    'network',
    'tunnels-config.tsx',
  )

  assert.doesNotMatch(source, /\.\.\.\(proxy \? \{ proxy \} : \{\}\)/)
  assert.doesNotMatch(source, /proxy:\s*['"]/)
})

test('existing tunnel proxy overrides are stripped before saving', () => {
  const source = read(
    'src',
    'components',
    'setting',
    'components',
    'network',
    'tunnels-config.tsx',
  )

  assert.match(source, /sanitizeTunnels/)
  assert.match(source, /const tunnels = sanitizeTunnels\(draftTunnels\)/)
  assert.match(source, /patchClash\(\{\s*tunnels\s*\}\)/)
})

test('tunnel copy no longer exposes proxy-specific labels', () => {
  for (const localeName of readdirSync(join(repoRoot, 'src', 'locales'))) {
    const source = read('src', 'locales', localeName, 'settings.json')

    assert.doesNotMatch(source, /"proxyGroup"/, localeName)
    assert.doesNotMatch(source, /"proxyNode"/, localeName)
    assert.doesNotMatch(source, /"tunnels"\s*:[\s\S]*"optional"/, localeName)
  }
})

test('generated i18n types no longer include tunnel proxy-specific keys', () => {
  for (const file of [
    ['src', 'types', 'generated', 'i18n-keys.ts'],
    ['src', 'types', 'generated', 'i18n-resources.ts'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /settings\.sections\.clash\.form\.fields\.tunnels\.proxyGroup/)
    assert.doesNotMatch(source, /settings\.sections\.clash\.form\.fields\.tunnels\.proxyNode/)
    assert.doesNotMatch(source, /settings\.sections\.clash\.form\.fields\.tunnels\.default/)
    assert.doesNotMatch(source, /settings\.sections\.clash\.form\.fields\.tunnels\.optional/)
    assert.doesNotMatch(source, /tunnels:\s*\{[\s\S]*proxyGroup:\s*string/)
    assert.doesNotMatch(source, /tunnels:\s*\{[\s\S]*proxyNode:\s*string/)
    assert.doesNotMatch(source, /tunnels:\s*\{[\s\S]*default:\s*string/)
    assert.doesNotMatch(source, /tunnels:\s*\{[\s\S]*optional:\s*string/)
  }
})

test('typed clash config no longer accepts tunnel proxy overrides', () => {
  const source = read('src', 'types', 'global.d.ts')

  assert.doesNotMatch(
    source,
    /tunnels\?:\s*\{[\s\S]*?proxy\?:[\s\S]*?\}\[\]/,
  )
})

test('runtime config strips stale tunnel proxy overrides', () => {
  const source = read('src-tauri', 'src', 'config', 'config.rs')

  assert.match(source, /strip_tunnel_proxy_overrides\(&mut config\)/)
  assert.match(source, /fn strip_tunnel_proxy_overrides\(config: &mut Mapping\)/)
  assert.match(source, /tunnel\.remove\("proxy"\)/)
  assert.doesNotMatch(source, /tunnels_need_validation/)
  assert.doesNotMatch(source, /collect_names/)
  assert.doesNotMatch(source, /valid\.contains/)
})
