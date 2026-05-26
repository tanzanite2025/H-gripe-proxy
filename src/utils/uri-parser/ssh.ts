import {
  decodeAndTrim,
  parseBoolOrPresence,
  parseInteger,
  parseIpVersion,
  parsePortOrDefault,
  parseQueryStringNormalized,
  parseUrlLike,
  safeDecodeURIComponent,
  splitOnce,
  stripUriScheme,
} from './helpers'

function parseCsvList(value: string | undefined): string[] | undefined {
  if (!value) return undefined
  const parsed = value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
  return parsed.length > 0 ? parsed : undefined
}

export function URI_SSH(line: string): IProxySshConfig {
  const afterScheme = stripUriScheme(line, 'ssh', 'Invalid ssh uri')
  if (!afterScheme) {
    throw new Error('Invalid ssh uri')
  }

  const {
    auth: authRaw,
    host: server,
    port,
    query: addons,
    fragment: nameRaw,
  } = parseUrlLike(afterScheme, { errorMessage: 'Invalid ssh uri' })

  if (!server) {
    throw new Error('Invalid ssh uri')
  }

  const portNum = parsePortOrDefault(port, 22)
  const auth = safeDecodeURIComponent(authRaw) ?? authRaw
  const decodedName = decodeAndTrim(nameRaw)
  const name = decodedName ?? `SSH ${server}:${portNum}`
  const proxy: IProxySshConfig = {
    type: 'ssh',
    name,
    server,
    port: portNum,
  }

  if (auth) {
    const [username, password] = splitOnce(auth, ':')
    if (username) {
      proxy.username = username
    }
    if (password !== undefined) {
      proxy.password = password
    }
  }

  const params = parseQueryStringNormalized(addons)
  if (!proxy.username) {
    proxy.username = params.username ?? params.user
  }
  if (proxy.password === undefined && params.password !== undefined) {
    proxy.password = params.password
  }

  for (const [key, value] of Object.entries(params)) {
    switch (key) {
      case 'private-key':
        if (!value) break
        proxy['private-key'] = value
        break
      case 'private-key-passphrase':
        if (!value) break
        proxy['private-key-passphrase'] = value
        break
      case 'host-key': {
        const hostKey = parseCsvList(value)
        if (hostKey) {
          proxy['host-key'] = hostKey
        }
        break
      }
      case 'host-key-algorithms': {
        const hostKeyAlgorithms = parseCsvList(value)
        if (hostKeyAlgorithms) {
          proxy['host-key-algorithms'] = hostKeyAlgorithms
        }
        break
      }
      case 'dialer-proxy':
        if (!value) break
        proxy['dialer-proxy'] = value
        break
      case 'interface-name':
        if (!value) break
        proxy['interface-name'] = value
        break
      case 'routing-mark': {
        const routingMark = parseInteger(value?.trim())
        if (routingMark !== undefined) {
          proxy['routing-mark'] = routingMark
        }
        break
      }
      case 'ip-version':
        proxy['ip-version'] = parseIpVersion(value)
        break
      case 'tfo':
        proxy.tfo = parseBoolOrPresence(value)
        break
      case 'mptcp':
        proxy.mptcp = parseBoolOrPresence(value)
        break
      default:
        break
    }
  }

  return proxy
}
