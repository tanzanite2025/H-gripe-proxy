import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

// Lock-step guard between the frontend proxy schema and the learn-gripe kernel.
//
// The frontend `IProxyConfig` union (src/types/global.d.ts) is the source of
// truth for which proxy `type` values the control plane can emit. The kernel's
// `ProxyType` enum (crates/learn-gripe/src/proxy.rs) must declare a variant for
// every one of them so config never fails to parse. This test fails loudly if
// the two drift apart — preventing both "frontend ships a type the kernel can't
// see" and "kernel keeps a dead variant the frontend dropped".

const repoRoot = process.cwd()
const read = (...segments) => readFileSync(join(repoRoot, ...segments), 'utf8')

function frontendProxyTypes() {
  const dts = read('src', 'types', 'global.d.ts')
  // The closing `type:` union inside `interface IProxyConfig ... { type: ... }`.
  const ifaceIndex = dts.indexOf('interface IProxyConfig')
  assert.notEqual(ifaceIndex, -1, 'IProxyConfig interface not found')
  const body = dts.slice(ifaceIndex)
  const typeIndex = body.indexOf('type:')
  assert.notEqual(typeIndex, -1, 'IProxyConfig narrowed `type:` union not found')
  // Capture from `type:` up to the next `}` that closes the interface.
  const union = body.slice(typeIndex, body.indexOf('}', typeIndex))
  const matches = [...union.matchAll(/'([a-z0-9-]+)'/g)].map((m) => m[1])
  return new Set(matches)
}

function kernelProxyTypes() {
  const rust = read('crates', 'learn-gripe', 'src', 'proxy.rs')
  const enumIndex = rust.indexOf('pub enum ProxyType')
  assert.notEqual(enumIndex, -1, 'ProxyType enum not found')
  const body = rust.slice(enumIndex, rust.indexOf('\n}', enumIndex))
  const matches = [...body.matchAll(/#\[serde\(rename = "([a-z0-9-]+)"\)\]/g)].map((m) => m[1])
  return new Set(matches)
}

test('learn-gripe ProxyType covers every frontend IProxyConfig type', () => {
  const frontend = frontendProxyTypes()
  const kernel = kernelProxyTypes()

  assert.ok(frontend.size >= 20, `expected a populated frontend union, got ${frontend.size}`)

  const missingInKernel = [...frontend].filter((t) => !kernel.has(t))
  assert.deepEqual(
    missingInKernel,
    [],
    `frontend proxy types missing a learn-gripe ProxyType variant: ${missingInKernel.join(', ')}`,
  )

  const danglingInKernel = [...kernel].filter((t) => !frontend.has(t))
  assert.deepEqual(
    danglingInKernel,
    [],
    `learn-gripe ProxyType variants not present in the frontend union: ${danglingInKernel.join(', ')}`,
  )
})

test('learn-gripe keeps an Unknown forward-compatibility catch-all', () => {
  const rust = read('crates', 'learn-gripe', 'src', 'proxy.rs')
  assert.match(rust, /#\[serde\(other\)\]\s*Unknown,/)
})
