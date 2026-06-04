import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const layoutPath = join(repoRoot, 'src', 'pages', '_layout', 'layout.tsx')
const switchPath = join(repoRoot, 'src', 'components', 'layout', 'global-outbound-switch.tsx')
const layoutStylePath = join(repoRoot, 'src', 'assets', 'styles', 'layout.scss')

test('global outbound switch is mounted in the center of the window header', () => {
  const layout = readFileSync(layoutPath, 'utf8')
  const styles = readFileSync(layoutStylePath, 'utf8')

  assert.match(layout, /GlobalOutboundSwitch/)
  assert.match(
    layout,
    /<div className="layout-header__center">[\s\S]*?<GlobalOutboundSwitch \/>[\s\S]*?<\/div>/,
  )
  assert.match(styles, /\.layout-header__center/)
  assert.match(styles, /left:\s*50%/)
  assert.match(styles, /transform:\s*translateX\(-50%\)/)
})

test('global outbound switch exposes direct and proxy-chain intents only', () => {
  assert.ok(existsSync(switchPath), 'global outbound switch component should exist')

  const source = readFileSync(switchPath, 'utf8')

  assert.match(source, /patchClashMode\('direct'\)/)
  assert.match(source, /patchClashMode\('rule'\)/)
  assert.doesNotMatch(source, /patchClashMode\('global'\)/)
  assert.match(source, /直连/)
  assert.match(source, /代理链路/)
  assert.match(source, /clashConfig\?\.mode/)
  assert.match(source, /runtimeConfig\?\.mode/)
})
