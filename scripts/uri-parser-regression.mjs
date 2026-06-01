import assert from 'node:assert/strict'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { createJiti } from 'jiti'

import { log_info, log_success } from './utils.mjs'

const currentDir = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(currentDir, '..')
const jiti = createJiti(import.meta.url, { moduleCache: false })
const parseUriModule = await jiti.import(
  path.join(rootDir, 'src/utils/parser/uri/index.ts'),
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
    label: 'ssh auth, key, host-key lists, and common fields',
    uri: 'ssh://alice:secret@example.com:22?private-key=LINE1%0ALINE2&private-key-passphrase=keypass&host-key=ssh-rsa%20AAA,ssh-ed25519%20BBB&host-key-algorithms=rsa,ssh-ed25519&ip-version=ipv6-prefer&dialer-proxy=ProxyA&interface-name=Ethernet%200&routing-mark=123&tfo=1&mptcp=1#SSH%20Primary',
    assert(parsed) {
      assert.equal(parsed.type, 'ssh')
      assert.equal(parsed.name, 'SSH Primary')
      assert.equal(parsed.server, 'example.com')
      assert.equal(parsed.port, 22)
      assert.equal(parsed.username, 'alice')
      assert.equal(parsed.password, 'secret')
      assert.equal(parsed['private-key'], 'LINE1\nLINE2')
      assert.equal(parsed['private-key-passphrase'], 'keypass')
      assert.deepEqual(parsed['host-key'], ['ssh-rsa AAA', 'ssh-ed25519 BBB'])
      assert.deepEqual(parsed['host-key-algorithms'], ['rsa', 'ssh-ed25519'])
      assert.equal(parsed['ip-version'], 'ipv6-prefer')
      assert.equal(parsed['dialer-proxy'], 'ProxyA')
      assert.equal(parsed['interface-name'], 'Ethernet 0')
      assert.equal(parsed['routing-mark'], 123)
      assert.equal(parsed.tfo, true)
      assert.equal(parsed.mptcp, true)
    },
  },
  {
    label: 'ssh query username fallback and default name',
    uri: 'ssh://fallback.example.com?username=bob&host-key-algorithms=rsa',
    assert(parsed) {
      assert.equal(parsed.type, 'ssh')
      assert.equal(parsed.name, 'SSH fallback.example.com:22')
      assert.equal(parsed.server, 'fallback.example.com')
      assert.equal(parsed.port, 22)
      assert.equal(parsed.username, 'bob')
      assert.deepEqual(parsed['host-key-algorithms'], ['rsa'])
    },
  },
  {
    label: 'snell auth, obfs-opts, and common fields',
    uri: 'snell://secret@example.com:443?version=3&udp=1&obfs=tls&obfs-host=cdn.example.com&ip-version=ipv4-prefer&dialer-proxy=ProxyB&interface-name=WLAN&routing-mark=321&tfo=1&mptcp=1#Snell%20TLS',
    assert(parsed) {
      assert.equal(parsed.type, 'snell')
      assert.equal(parsed.name, 'Snell TLS')
      assert.equal(parsed.server, 'example.com')
      assert.equal(parsed.port, 443)
      assert.equal(parsed.psk, 'secret')
      assert.equal(parsed.version, 3)
      assert.equal(parsed.udp, true)
      assert.deepEqual(parsed['obfs-opts'], {
        mode: 'tls',
        host: 'cdn.example.com',
      })
      assert.equal(parsed['ip-version'], 'ipv4-prefer')
      assert.equal(parsed['dialer-proxy'], 'ProxyB')
      assert.equal(parsed['interface-name'], 'WLAN')
      assert.equal(parsed['routing-mark'], 321)
      assert.equal(parsed.tfo, true)
      assert.equal(parsed.mptcp, true)
    },
  },
  {
    label: 'snell query psk fallback and default name',
    uri: 'snell://fallback-snell.example.com?psk=query-secret&mode=http&host=www.example.com',
    assert(parsed) {
      assert.equal(parsed.type, 'snell')
      assert.equal(parsed.name, 'Snell fallback-snell.example.com:443')
      assert.equal(parsed.server, 'fallback-snell.example.com')
      assert.equal(parsed.port, 443)
      assert.equal(parsed.psk, 'query-secret')
      assert.deepEqual(parsed['obfs-opts'], {
        mode: 'http',
        host: 'www.example.com',
      })
    },
  },
  {
    label: 'sudoku official short link httpmask mapping',
    uri: 'sudoku://eyJoIjoiZWRnZS5leGFtcGxlLm5ldCIsInAiOjQ0MywiayI6ImRlYWRiZWVmIiwiYSI6ImVudHJvcHkiLCJlIjoiYWVzLTEyOC1nY20iLCJ4Ijp0cnVlLCJ0IjoieHB4dnZwdnYiLCJoZCI6ZmFsc2UsImhtIjoiYXV0byIsImh0Ijp0cnVlLCJoaCI6ImNkbi5leGFtcGxlLmNvbSIsImh4IjoiYXV0byIsImh5IjoiYWFiYmNjIn0#Sudoku%20Auto',
    assert(parsed) {
      assert.equal(parsed.type, 'sudoku')
      assert.equal(parsed.name, 'Sudoku Auto')
      assert.equal(parsed.server, 'edge.example.net')
      assert.equal(parsed.port, 443)
      assert.equal(parsed.key, 'deadbeef')
      assert.equal(parsed['aead-method'], 'aes-128-gcm')
      assert.equal(parsed['table-type'], 'prefer_entropy')
      assert.equal(parsed['enable-pure-downlink'], false)
      assert.equal(parsed['custom-table'], 'xpxvvpvv')
      assert.equal(parsed['padding-min'], 5)
      assert.equal(parsed['padding-max'], 15)
      assert.deepEqual(parsed.httpmask, {
        disable: false,
        mode: 'auto',
        tls: true,
        host: 'cdn.example.com',
        multiplex: 'auto',
        'path-root': 'aabbcc',
      })
    },
  },
  {
    label: 'sudoku directional ascii and default fallbacks',
    uri: 'sudoku://eyJoIjoiMjAwMTpkYjg6OjEiLCJwIjo0NDMsImsiOiJmZWVkZmFjZSIsImEiOiJ1cF9hc2NpaV9kb3duX2VudHJvcHkiLCJ0cyI6WyJ4cHh2dnB2diIsInZ4cHZ4dnZwIl0sImhkIjp0cnVlfQ',
    assert(parsed) {
      assert.equal(parsed.type, 'sudoku')
      assert.equal(parsed.name, 'Sudoku 2001:db8::1:443')
      assert.equal(parsed.server, '2001:db8::1')
      assert.equal(parsed.port, 443)
      assert.equal(parsed.key, 'feedface')
      assert.equal(parsed['aead-method'], 'none')
      assert.equal(parsed['table-type'], 'up_ascii_down_entropy')
      assert.equal(parsed['enable-pure-downlink'], true)
      assert.deepEqual(parsed['custom-tables'], ['xpxvvpvv', 'vxpvxvvp'])
      assert.deepEqual(parsed.httpmask, {
        disable: true,
      })
    },
  },
  {
    label: 'mierus single binding maps to mihomo mieru node',
    uri: 'mierus://baozi:manlianpenfen@1.2.3.4?profile=default&multiplexing=MULTIPLEXING_HIGH&port=6666&protocol=TCP&traffic-pattern=CCoQARoECAEQCiIYCAMQASoIMDAwMTAyMDMqCDA0MDUwNjA3#Mieru%20TCP',
    assert(parsed) {
      assert.equal(parsed.type, 'mieru')
      assert.equal(parsed.name, 'Mieru TCP')
      assert.equal(parsed.server, '1.2.3.4')
      assert.equal(parsed.port, 6666)
      assert.equal(parsed.transport, 'TCP')
      assert.equal(parsed.username, 'baozi')
      assert.equal(parsed.password, 'manlianpenfen')
      assert.equal(parsed.multiplexing, 'MULTIPLEXING_HIGH')
      assert.equal(
        parsed['traffic-pattern'],
        'CCoQARoECAEQCiIYCAMQASoIMDAwMTAyMDMqCDA0MDUwNjA3',
      )
    },
  },
  {
    label: 'mierus port-range and udp transport mapping',
    uri: 'mierus://user:pass@edge.example.net?profile=edge&port=9998-9999&protocol=UDP&multiplexing=MULTIPLEXING_LOW',
    assert(parsed) {
      assert.equal(parsed.type, 'mieru')
      assert.equal(parsed.name, 'edge')
      assert.equal(parsed.server, 'edge.example.net')
      assert.equal(parsed['port-range'], '9998-9999')
      assert.equal(parsed.transport, 'UDP')
      assert.equal(parsed.username, 'user')
      assert.equal(parsed.password, 'pass')
      assert.equal(parsed.multiplexing, 'MULTIPLEXING_LOW')
    },
  },
  {
    label: 'mierus multiple bindings are rejected',
    uri: 'mierus://baozi:manlianpenfen@1.2.3.4?profile=default&port=6666&port=6489&protocol=TCP&protocol=UDP',
    assert() {
      assert.throws(
        () => parseUri('mierus://baozi:manlianpenfen@1.2.3.4?profile=default&port=6666&port=6489&protocol=TCP&protocol=UDP'),
        /multiple port\/protocol bindings/i,
      )
    },
  },
  {
    label: 'mieru standard full-config link is explicitly unsupported',
    uri: 'mieru://CpsBCgdkZWZhdWx0ElgKBWJhb3ppEg1tYW5saWFucGVuZmVuGkA0MGFiYWM0MGY1OWRhNTVkYWQ2YTk5ODMxYTUxMTY1MjJmYmM4MGUzODViYjFhYjE0ZGM1MmRiMzY4ZjczOGE0Gi8SCWxvY2FsaG9zdBoFCIo0EAIaDRACGgk5OTk5LTk5OTkaBQjZMhABGgUIoCYQASD4CioCCAQSB2RlZmF1bHQYnUYguAgwBTgA',
    assert() {
      assert.throws(
        () => parseUri('mieru://CpsBCgdkZWZhdWx0ElgKBWJhb3ppEg1tYW5saWFucGVuZmVuGkA0MGFiYWM0MGY1OWRhNTVkYWQ2YTk5ODMxYTUxMTY1MjJmYmM4MGUzODViYjFhYjE0ZGM1MmRiMzY4ZjczOGE0Gi8SCWxvY2FsaG9zdBoFCIo0EAIaDRACGgk5OTk5LTk5OTkaBQjZMhABGgUIoCYQASD4CioCCAQSB2RlZmF1bHQYnUYguAgwBTgA'),
        /unsupported mieru uri/i,
      )
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
  if (testCase.assert.length === 0) {
    testCase.assert()
    log_success(`PASS ${testCase.label}`)
    continue
  }

  const parsed = parseUri(testCase.uri)
  testCase.assert(parsed)
  log_success(`PASS ${testCase.label}`)
}

log_success(`URI parser regression samples passed: ${cases.length}`)
