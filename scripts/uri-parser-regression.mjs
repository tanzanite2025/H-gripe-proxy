import assert from 'node:assert/strict'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { createJiti } from 'jiti'

import { log_info, log_success } from './utils.mjs'

const currentDir = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(currentDir, '..')
const jiti = createJiti(import.meta.url, { moduleCache: false })
const parseUriModule = await jiti.import(
  path.join(rootDir, 'src/utils/uri-parser/index.ts'),
)
const parseUri = parseUriModule.default ?? parseUriModule

const cases = [
  {
    label: 'trojan-go ws defaults and reality mapping',
    uri: 'trojan-go://password@example.com:443?type=ws&path=%2Fwebsocket&allowInsecure=1&pbk=PUBLICKEY&sid=SHORTID#Trojan-Go%20WS',
    assert(parsed) {
      assert.equal(parsed.type, 'trojan')
      assert.equal(parsed.name, 'Trojan-Go WS')
      assert.equal(parsed.server, 'example.com')
      assert.equal(parsed.port, 443)
      assert.equal(parsed.password, 'password')
      assert.equal(parsed.network, 'ws')
      assert.equal(parsed.sni, 'example.com')
      assert.equal(parsed['skip-cert-verify'], true)
      assert.deepEqual(parsed['ws-opts'], {
        path: '/websocket',
        headers: { Host: 'example.com' },
      })
      assert.deepEqual(parsed['reality-opts'], {
        'public-key': 'PUBLICKEY',
        'short-id': 'SHORTID',
      })
    },
  },
  {
    label: 'vless xhttp transport mapping',
    uri: 'vless://123e4567-e89b-12d3-a456-426614174000@origin.example.com:443?security=tls&type=xhttp&host=cdn.example.com&path=%2Fxhttp&mode=packet-up#VLESS%20XHTTP',
    assert(parsed) {
      assert.equal(parsed.type, 'vless')
      assert.equal(parsed.name, 'VLESS XHTTP')
      assert.equal(parsed.server, 'origin.example.com')
      assert.equal(parsed.port, 443)
      assert.equal(parsed.uuid, '123e4567-e89b-12d3-a456-426614174000')
      assert.equal(parsed.tls, true)
      assert.equal(parsed.network, 'xhttp')
      assert.equal(parsed.servername, 'cdn.example.com')
      assert.deepEqual(parsed['xhttp-opts'], {
        host: 'cdn.example.com',
        path: '/xhttp',
        mode: 'packet-up',
      })
    },
  },
  {
    label: 'vless splithttp alias maps to xhttp',
    uri: 'vless://123e4567-e89b-12d3-a456-426614174000@split.example.com:443?security=reality&type=splithttp&host=cdn-split.example.com&path=%2Fsplit&mode=auto&pbk=PUBKEY&sid=SHORT#SplitHTTP',
    assert(parsed) {
      assert.equal(parsed.type, 'vless')
      assert.equal(parsed.name, 'SplitHTTP')
      assert.equal(parsed.server, 'split.example.com')
      assert.equal(parsed.port, 443)
      assert.equal(parsed.network, 'xhttp')
      assert.equal(parsed.tls, true)
      assert.equal(parsed.servername, 'cdn-split.example.com')
      assert.deepEqual(parsed['xhttp-opts'], {
        host: 'cdn-split.example.com',
        path: '/split',
        mode: 'auto',
      })
      assert.deepEqual(parsed['reality-opts'], {
        'public-key': 'PUBKEY',
        'short-id': 'SHORT',
      })
    },
  },
]

log_info('Running URI parser regression samples...')

for (const testCase of cases) {
  const parsed = parseUri(testCase.uri)
  testCase.assert(parsed)
  log_success(`PASS ${testCase.label}`)
}

log_success(`URI parser regression samples passed: ${cases.length}`)
