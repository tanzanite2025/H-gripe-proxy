import {
  decodeAndTrim,
  normalizeUriAndGetScheme,
  parseBoolOrPresence,
  parseInteger,
  parseIpVersion,
  parseRequiredPort,
  parseUrlLike,
  safeDecodeURIComponent,
  splitOnce,
  stripUriScheme,
} from './helpers'

function parseRepeatedQueryNormalized(
  query: string | undefined,
): Array<[string, string | undefined]> {
  const entries: Array<[string, string | undefined]> = []
  if (!query) return entries

  for (const part of query.split('&')) {
    if (!part) continue
    const [keyRaw, valueRaw] = splitOnce(part, '=')
    const key = keyRaw.trim().replace(/_/g, '-').toLowerCase()
    if (!key) continue
    entries.push([
      key,
      valueRaw === undefined
        ? undefined
        : (safeDecodeURIComponent(valueRaw) ?? valueRaw),
    ])
  }

  return entries
}

function parsePortOrRange(value: string): { port?: number; portRange?: string } {
  const trimmed = value.trim()
  const [startRaw, endRaw] = splitOnce(trimmed, '-')
  if (endRaw !== undefined) {
    const start = parseRequiredPort(startRaw, 'Invalid mieru uri: invalid port')
    const end = parseRequiredPort(endRaw, 'Invalid mieru uri: invalid port')
    if (start > end) {
      throw new Error('Invalid mieru uri: invalid port range')
    }
    return { portRange: `${start}-${end}` }
  }
  return { port: parseRequiredPort(trimmed, 'Invalid mieru uri: invalid port') }
}

function parseMieruTransport(value: string | undefined): MieruTransport | undefined {
  const normalized = value?.trim().toUpperCase()
  switch (normalized) {
    case 'TCP':
    case 'UDP':
      return normalized
    default:
      return undefined
  }
}

function parseMieruMultiplexing(
  value: string | undefined,
): MieruMultiplexing | undefined {
  const normalized = value?.trim().toUpperCase()
  switch (normalized) {
    case 'MULTIPLEXING_OFF':
    case 'MULTIPLEXING_LOW':
    case 'MULTIPLEXING_MIDDLE':
    case 'MULTIPLEXING_HIGH':
      return normalized
    default:
      return undefined
  }
}

export function URI_Mieru(line: string): IProxyMieruConfig {
  const { scheme } = normalizeUriAndGetScheme(line)
  if (scheme === 'mieru') {
    throw new Error(
      'Unsupported mieru uri: standard mieru:// links contain full client configuration; use mierus:// simple share links instead',
    )
  }

  const afterScheme = stripUriScheme(line, 'mierus', 'Invalid mieru uri')
  if (!afterScheme) {
    throw new Error('Invalid mieru uri')
  }

  const {
    auth: authRaw,
    host: server,
    port: authorityPort,
    query: addons,
    fragment: nameRaw,
  } = parseUrlLike(afterScheme, {
    requireAuth: true,
    errorMessage: 'Invalid mieru uri',
  })

  if (!server) {
    throw new Error('Invalid mieru uri')
  }

  const auth = safeDecodeURIComponent(authRaw) ?? authRaw
  const [username, password] = splitOnce(auth, ':')
  if (!username || password === undefined) {
    throw new Error('Invalid mieru uri')
  }

  let profile: string | undefined
  let transportAlias: MieruTransport | undefined
  const ports: string[] = []
  const protocols: string[] = []
  const proxy: IProxyMieruConfig = {
    type: 'mieru',
    name: '',
    server,
    username,
    password,
  }

  for (const [key, value] of parseRepeatedQueryNormalized(addons)) {
    switch (key) {
      case 'profile':
        if (value) profile = value
        break
      case 'multiplexing': {
        const multiplexing = parseMieruMultiplexing(value)
        if (multiplexing) {
          proxy.multiplexing = multiplexing
        }
        break
      }
      case 'traffic-pattern':
        if (value) {
          proxy['traffic-pattern'] = value
        }
        break
      case 'port':
      case 'port-range':
        if (value) {
          ports.push(value)
        }
        break
      case 'protocol':
        if (value) {
          protocols.push(value)
        }
        break
      case 'transport': {
        const transport = parseMieruTransport(value)
        if (transport) {
          transportAlias = transport
        }
        break
      }
      case 'udp':
        proxy.udp = parseBoolOrPresence(value)
        break
      case 'dialer-proxy':
        if (value) proxy['dialer-proxy'] = value
        break
      case 'interface-name':
        if (value) proxy['interface-name'] = value
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
      case 'mtu':
      case 'handshake-mode':
        break
      default:
        break
    }
  }

  if (authorityPort) {
    if (ports.length > 0) {
      throw new Error('Invalid mieru uri: conflicting authority port and port parameters')
    }
    ports.push(authorityPort)
  }

  if (transportAlias && protocols.length === 0) {
    protocols.push(transportAlias)
  }

  if (ports.length === 0) {
    throw new Error('Invalid mieru uri: missing port')
  }
  if (protocols.length === 0) {
    throw new Error('Invalid mieru uri: missing protocol')
  }
  if (ports.length !== protocols.length) {
    throw new Error('Invalid mieru uri: port and protocol counts differ')
  }
  if (ports.length !== 1) {
    throw new Error(
      'Unsupported mieru uri: multiple port/protocol bindings are not representable as a single Mihomo mieru node',
    )
  }

  const transport = parseMieruTransport(protocols[0])
  if (!transport) {
    throw new Error('Invalid mieru uri: invalid protocol')
  }
  proxy.transport = transport

  const binding = parsePortOrRange(ports[0])
  if (binding.port !== undefined) {
    proxy.port = binding.port
  }
  if (binding.portRange) {
    proxy['port-range'] = binding.portRange
  }

  const decodedName = decodeAndTrim(nameRaw)
  proxy.name =
    decodedName ??
    profile ??
    (binding.portRange
      ? `Mieru ${server}:${binding.portRange}`
      : `Mieru ${server}:${binding.port}`)

  return proxy
}
