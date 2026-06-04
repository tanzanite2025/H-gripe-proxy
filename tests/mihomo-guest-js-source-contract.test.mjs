import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const pluginRoot = join(repoRoot, 'crates', 'tauri-plugin-mihomo')
const guestIndexPath = join(pluginRoot, 'guest-js', 'index.ts')
const distIndexTypesPath = join(pluginRoot, 'dist-js', 'index.d.ts')
const guestBindingsIndexPath = join(pluginRoot, 'guest-js', 'bindings', 'index.ts')
const distBindingsIndexPath = join(pluginRoot, 'dist-js', 'bindings', 'index.d.ts')

const read = (path) => readFileSync(path, 'utf8')
const exportedFunctions = (source) =>
  [...source.matchAll(/export (?:declare )?(?:async )?function (\w+)\(/g)].map(
    ([, name]) => name,
  )
const bindingExports = (source) =>
  [...source.matchAll(/export \* from "\.\/([^"]+)"/g)].map(([, name]) => name)

test('mihomo guest-js source keeps public API functions present in dist types', () => {
  const guestFunctions = new Set(exportedFunctions(read(guestIndexPath)))
  const distFunctions = exportedFunctions(read(distIndexTypesPath))

  for (const name of distFunctions) {
    assert.ok(guestFunctions.has(name), `guest-js/index.ts should export ${name}`)
  }
})

test('mihomo guest-js binding barrel keeps generated dist binding exports', () => {
  const guestBindings = new Set(bindingExports(read(guestBindingsIndexPath)))
  const distBindings = bindingExports(read(distBindingsIndexPath))

  for (const name of distBindings) {
    assert.ok(
      guestBindings.has(name),
      `guest-js/bindings/index.ts should export ${name}`,
    )
  }
})
