import {
  decodeAndTrim,
  getIfNotBlank,
  parseBoolOrPresence,
  parsePortOrDefault,
  parseQueryStringNormalized,
  parseUrlLike,
  safeDecodeURIComponent,
  stripUriScheme,
} from './helpers'
import { buildGrpcOptions, buildWsOptions } from './transport'

type TrojanScheme = 'trojan' | 'trojan-go'

export function parseTrojanUri(
  line: string,
  expectedSchemes: TrojanScheme | readonly TrojanScheme[],
  errorMessage = 'Invalid trojan uri',
): IProxyTrojanConfig {
  const afterScheme = stripUriScheme(line, expectedSchemes, errorMessage)
  if (!afterScheme) {
    throw new Error(errorMessage)
  }
  const {
    auth: passwordRaw,
    host: server,
    port,
    query: addons,
    fragment: nameRaw,
  } = parseUrlLike(afterScheme, {
    requireAuth: true,
    errorMessage,
  })
  const schemes = Array.isArray(expectedSchemes)
    ? expectedSchemes
    : [expectedSchemes]
  const isTrojanGo = schemes.includes('trojan-go')
  const portNum = parsePortOrDefault(port, 443)
  const password = safeDecodeURIComponent(passwordRaw) ?? passwordRaw
  const name = decodeAndTrim(nameRaw) ?? `Trojan ${server}:${portNum}`
  const proxy: IProxyTrojanConfig = {
    type: 'trojan',
    name,
    server,
    port: portNum,
    password,
  }

  const params = parseQueryStringNormalized(addons)

  let network = params.type?.toLowerCase()
  if (network === 'websocket') network = 'ws'
  if (network === 'original') network = undefined
  if (network && ['ws', 'grpc', 'h2', 'tcp'].includes(network)) {
    proxy.network = network as NetworkType
  }

  const host = getIfNotBlank(params.host) ?? (isTrojanGo ? server : undefined)
  const path = getIfNotBlank(params.path)

  if (params.alpn) {
    proxy.alpn = params.alpn.split(',')
  }
  if (params.sni) {
    proxy.sni = params.sni
  } else if (isTrojanGo) {
    proxy.sni = server
  }
  if (Object.prototype.hasOwnProperty.call(params, 'skip-cert-verify')) {
    proxy['skip-cert-verify'] = parseBoolOrPresence(params['skip-cert-verify'])
  } else if (Object.prototype.hasOwnProperty.call(params, 'allowInsecure')) {
    proxy['skip-cert-verify'] = parseBoolOrPresence(params.allowInsecure)
  }

  proxy.fingerprint = params.fingerprint ?? params.fp

  if (params.pbk || params.sid) {
    const realityOpts: RealityOptions = {}
    if (params.pbk) {
      realityOpts['public-key'] = params.pbk
    }
    if (params.sid) {
      realityOpts['short-id'] = params.sid
    }
    if (Object.keys(realityOpts).length > 0) {
      proxy['reality-opts'] = realityOpts
    }
  }

  if (params.encryption) {
    const encryption = params.encryption.split(';')
    if (encryption.length === 3) {
      proxy['ss-opts'] = {
        enabled: true,
        method: encryption[1],
        password: encryption[2],
      }
    }
  }

  if (params['client-fingerprint']) {
    proxy['client-fingerprint'] = params[
      'client-fingerprint'
    ] as ClientFingerprint
  }

  if (proxy.network === 'ws') {
    const wsOpts = buildWsOptions(host, path)
    if (wsOpts) {
      proxy['ws-opts'] = wsOpts
    }
  } else if (proxy.network === 'grpc') {
    const grpcOpts = buildGrpcOptions(path)
    if (grpcOpts) {
      proxy['grpc-opts'] = grpcOpts
    }
  }

  return proxy
}

export function URI_Trojan(line: string): IProxyTrojanConfig {
  return parseTrojanUri(line, 'trojan')
}
