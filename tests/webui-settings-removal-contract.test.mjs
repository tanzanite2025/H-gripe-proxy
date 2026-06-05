import assert from 'node:assert/strict'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const read = (...segments) => readFileSync(join(repoRoot, ...segments), 'utf8')
const exists = (...segments) => existsSync(join(repoRoot, ...segments))

test('clash settings no longer exposes the Web UI settings dialog', () => {
  const source = read('src', 'components', 'setting', 'setting-clash.tsx')

  assert.doesNotMatch(source, /WebUIViewer/)
  assert.doesNotMatch(source, /webRef/)
  assert.doesNotMatch(source, /settings\.sections\.clash\.form\.fields\.webUI/)
})

test('Web UI settings components are removed', () => {
  assert.equal(exists('src', 'components', 'setting', 'components', 'webui', 'webui-config.tsx'), false)
  assert.equal(exists('src', 'components', 'setting', 'components', 'webui', 'webui-item.tsx'), false)
})

test('typed verge config no longer accepts a Web UI list', () => {
  for (const file of [
    ['src', 'types', 'global.d.ts'],
    ['src-tauri', 'src', 'config', 'verge.rs'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /web_ui_list/)
  }
})

test('generic web URL opening remains available for non-Web-UI links', () => {
  const cmds = read('src', 'services', 'cmds.ts')
  const appCmd = read('src-tauri', 'src', 'cmd', 'app.rs')
  const appLib = read('src-tauri', 'src', 'lib.rs')

  assert.match(cmds, /openWebUrl/)
  assert.match(appCmd, /open_web_url/)
  assert.match(appLib, /cmd::open_web_url/)
})

test('frontend locale resources no longer keep Web UI settings copy', () => {
  for (const localeName of readdirSync(join(repoRoot, 'src', 'locales'))) {
    const source = read('src', 'locales', localeName, 'settings.json')

    assert.doesNotMatch(source, /"webUI"/, localeName)
  }
})

test('generated i18n types no longer include Web UI settings keys', () => {
  for (const file of [
    ['src', 'types', 'generated', 'i18n-keys.ts'],
    ['src', 'types', 'generated', 'i18n-resources.ts'],
  ]) {
    const source = read(...file)

    assert.doesNotMatch(source, /webUI/)
  }
})
