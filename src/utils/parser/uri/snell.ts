import {
  decodeAndTrim,
  parseBoolOrPresence,
  parseInteger,
  parseIpVersion,
  parsePortOrDefault,
  parseQueryStringNormalized,
  parseUrlLike,
  safeDecodeURIComponent,
  stripUriScheme,
} from './helpers'

export function URI_Snell(line: string): IProxySnellConfig {
  const afterScheme = stripUriScheme(line, 'snell', 'Invalid snell uri')
  if (!afterScheme) {
    throw new Error('Invalid snell uri')
  }

  const {
    auth: pskRaw,
    host: server,
    port,
    query: addons,
    fragment: nameRaw,
  } = parseUrlLike(afterScheme, { errorMessage: 'Invalid snell uri' })

  if (!server) {
    throw new Error('Invalid snell uri')
  }

  const portNum = parsePortOrDefault(port, 443)
  const psk = safeDecodeURIComponent(pskRaw) ?? pskRaw
  const decodedName = decodeAndTrim(nameRaw)
  const name = decodedName ?? `Snell ${server}:${portNum}`
  const proxy: IProxySnellConfig = {
    type: 'snell',
    name,
    server,
    port: portNum,
  }

  if (psk) {
    proxy.psk = psk
  }

  const params = parseQueryStringNormalized(addons)
  if (proxy.psk === undefined && params.psk !== undefined) {
    proxy.psk = params.psk
  }

  for (const [key, value] of Object.entries(params)) {
    switch (key) {
      case 'version': {
        const version = parseInteger(value?.trim())
        if (version !== undefined && version >= 1 && version <= 3) {
          proxy.version = version
        }
        break
      }
      case 'udp':
      case 'udp-relay':
        proxy.udp = parseBoolOrPresence(value)
        break
      case 'obfs':
      case 'mode': {
        if (value === 'http' || value === 'tls') {
          proxy['obfs-opts'] = {
            ...(proxy['obfs-opts'] ?? {}),
            mode: value,
          }
        }
        break
      }
      case 'obfs-host':
      case 'host':
        if (!value) break
        proxy['obfs-opts'] = {
          ...(proxy['obfs-opts'] ?? {}),
          host: value,
        }
        break
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
