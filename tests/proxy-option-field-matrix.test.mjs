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

const optionSources = new Map(
  fs
    .readdirSync(path.join(root, 'mihomo/adapter/outbound'))
    .filter((file) => file.endsWith('.go') && !file.endsWith('_test.go'))
    .map((file) => [
      file,
      fs.readFileSync(path.join(root, 'mihomo/adapter/outbound', file), 'utf8'),
    ]),
)

const proxyInterfaceByType = {
  ss: 'IProxyShadowsocksConfig',
  ssr: 'IProxyshadowsocksRConfig',
  socks5: 'IProxySocks5Config',
  http: 'IProxyHttpConfig',
  vmess: 'IProxyVmessConfig',
  vless: 'IProxyVlessConfig',
  snell: 'IProxySnellConfig',
  trojan: 'IProxyTrojanConfig',
  hysteria: 'IProxyHysteriaConfig',
  hysteria2: 'IProxyHysteria2Config',
  wireguard: 'IProxyWireguardConfig',
  tuic: 'IProxyTuicConfig',
  'gost-relay': 'IProxyGostRelayConfig',
  direct: 'IProxyDirectConfig',
  dns: 'IProxyDnsConfig',
  reject: 'IProxyRejectConfig',
  ssh: 'IProxySshConfig',
  mieru: 'IProxyMieruConfig',
  anytls: 'IProxyAnyTLSConfig',
  sudoku: 'IProxySudokuConfig',
  masque: 'IProxyMasqueConfig',
  trusttunnel: 'IProxyTrustTunnelConfig',
  openvpn: 'IProxyOpenVPNConfig',
  tailscale: 'IProxyTailscaleConfig',
}

const intentionallyNestedProxyFields = {
  ss: new Set(['plugin-opts']),
  trojan: new Set([
    'reality-opts',
    'grpc-opts',
    'ws-opts',
    'ss-opts',
    'client-padding',
    'server-padding',
    'password-pipeline',
  ]),
  vmess: new Set(['h2-opts', 'http-opts', 'grpc-opts', 'ws-opts', 'reality-opts']),
  vless: new Set(['h2-opts', 'http-opts', 'grpc-opts', 'ws-opts', 'reality-opts']),
  wireguard: new Set(['peers', 'amnezia-wg-option']),
  hysteria2: new Set(['realm']),
  sudoku: new Set(['httpmask']),
  anytls: new Set(['ech-opts']),
  trusttunnel: new Set(['ech-opts']),
  direct: new Set(['udp']),
}

const knownRemainingFieldGaps = []

function findStructSource(structName) {
  for (const [file, source] of optionSources) {
    const structBlock = source.match(
      new RegExp(`type ${structName} struct \\{([\\s\\S]*?)\\n\\}`),
    )
    if (structBlock) {
      return { file, block: structBlock[1] }
    }
  }
  return undefined
}

function collectProxyTags(structName) {
  const structSource = findStructSource(structName)
  assert.ok(structSource, `${structName} should exist`)

  return [...structSource.block.matchAll(/`proxy:"([^",]+)[^"]*"`/g)]
    .map((match) => match[1])
    .filter((field) => field !== 'name')
    .sort()
}

function collectInterfaceFields(interfaceName) {
  const interfaceBlock = globalTypes.match(
    new RegExp(`interface ${interfaceName}[^\\{]*\\{([\\s\\S]*?)\\n\\}`),
  )
  assert.ok(interfaceBlock, `${interfaceName} should exist`)

  return [...interfaceBlock[1].matchAll(/^\s*(?:'([^']+)'|([A-Za-z][A-Za-z0-9_-]*))\??:/gm)]
    .map((match) => match[1] ?? match[2])
    .filter((field) => !['name', 'type'].includes(field))
    .sort()
}

function collectParserProxyOptions() {
  return [...parser.matchAll(/case "([^"]+)":[\s\S]*?&outbound\.([A-Za-z0-9]+Option)\{/g)]
    .map((match) => ({
      type: match[1],
      option: match[2],
      interfaceName: proxyInterfaceByType[match[1]],
    }))
}

test('frontend proxy config fields cover mihomo proxy option tags', () => {
  const gaps = []

  for (const item of collectParserProxyOptions()) {
    assert.ok(item.interfaceName, `missing frontend interface mapping for ${item.type}`)
    const ignored = intentionallyNestedProxyFields[item.type] ?? new Set()
    const coreFields = collectProxyTags(item.option).filter(
      (field) => !ignored.has(field),
    )
    const frontendFields = collectInterfaceFields(item.interfaceName)
    const missing = coreFields.filter((field) => !frontendFields.includes(field))

    if (missing.length > 0) {
      gaps.push(`${item.type} (${item.option} -> ${item.interfaceName}): ${missing.join(', ')}`)
    }
  }

  assert.deepEqual(
    gaps,
    knownRemainingFieldGaps,
    'frontend proxy config field gaps should stay explicit while audit rounds remove them',
  )
})
