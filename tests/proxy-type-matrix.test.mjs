import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { test } from 'node:test'

const root = path.resolve(import.meta.dirname, '..')
const parser = fs.readFileSync(
  path.join(root, 'mihomo/adapter/parser.go'),
  'utf8',
)
const globalTypes = fs.readFileSync(
  path.join(root, 'src/types/global.d.ts'),
  'utf8',
)

function collectCoreProxyTypes() {
  return [...parser.matchAll(/case "([^"]+)":/g)]
    .map((match) => match[1])
    .sort()
}

function collectFrontendProxyTypes() {
  const typeBlock = globalTypes.match(/interface IProxyConfig[\s\S]*?type:\s*([\s\S]*?)\n}/)
  assert.ok(typeBlock, 'IProxyConfig type union should exist')
  return [...typeBlock[1].matchAll(/\|\s*'([^']+)'/g)]
    .map((match) => match[1])
    .sort()
}

test('frontend proxy type union covers mihomo parser types', () => {
  const coreTypes = collectCoreProxyTypes()
  const frontendTypes = collectFrontendProxyTypes()

  assert.deepEqual(
    frontendTypes.filter((type) => coreTypes.includes(type)),
    coreTypes,
    'IProxyConfig.type should include every proxy type parsed by mihomo',
  )
})
