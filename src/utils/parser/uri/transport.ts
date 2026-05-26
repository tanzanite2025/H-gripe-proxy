import { firstString, getIfNotBlank } from './helpers'

function normalizeTransportString(value: unknown): string | undefined {
  if (value === null || value === undefined) return undefined
  const raw = String(value).trim()
  return raw ? raw : undefined
}

function toStringArray(value: unknown): string[] | undefined {
  if (value === null || value === undefined) return undefined
  const values = (Array.isArray(value) ? value : [value])
    .map((item) => normalizeTransportString(item))
    .filter((item): item is string => Boolean(item))
  return values.length > 0 ? values : undefined
}

export function parseHostFromMaybeJson(value: unknown): unknown {
  const raw = normalizeTransportString(value)
  if (!raw) return value
  try {
    const parsed = JSON.parse(raw) as { Host?: unknown } | null
    return parsed?.Host ?? value
  } catch {
    return value
  }
}

export function getTransportHostFirst(value: unknown): string | undefined {
  return getIfNotBlank(firstString(value))
}

export function getTransportPathFirst(value: unknown): string | undefined {
  return getIfNotBlank(firstString(value))
}

export function buildGrpcOptions(path: unknown): GrpcOptions | undefined {
  const serviceName = getTransportPathFirst(path)
  return serviceName ? { 'grpc-service-name': serviceName } : undefined
}

export function buildH2Options(
  host: unknown,
  path: unknown,
): H2Options | undefined {
  const hostValue = getTransportHostFirst(host)
  const pathValue = getTransportPathFirst(path)
  if (!hostValue && !pathValue) return undefined
  const h2Opts: H2Options = {}
  if (hostValue) h2Opts.host = hostValue
  if (pathValue) h2Opts.path = pathValue
  return h2Opts
}

export function buildHttpOptions(
  host: unknown,
  path: unknown,
  options: { defaultPath?: string[] } = {},
): HttpOptions | undefined {
  const hostList = toStringArray(host)
  let pathList = toStringArray(path)
  if ((!pathList || pathList.length === 0) && options.defaultPath) {
    pathList = options.defaultPath
  }
  if ((!hostList || hostList.length === 0) && (!pathList || pathList.length === 0)) {
    return undefined
  }
  const httpOpts: HttpOptions = {}
  if (pathList && pathList.length > 0) {
    httpOpts.path = pathList
  }
  if (hostList && hostList.length > 0) {
    httpOpts.headers = { Host: hostList }
  }
  return httpOpts
}

export function buildWsOptions(
  host: unknown,
  path: unknown,
  options: { preferJsonHeaders?: boolean; httpupgrade?: boolean } = {},
): WsOptions | undefined {
  const pathValue = getTransportPathFirst(path)
  const hostValue = getTransportHostFirst(host)
  let headers: Record<string, string> | undefined
  if (hostValue) {
    if (options.preferJsonHeaders) {
      try {
        const parsed = JSON.parse(hostValue) as Record<string, unknown>
        const normalized: Record<string, string> = {}
        for (const [key, value] of Object.entries(parsed)) {
          const stringValue = normalizeTransportString(value)
          if (stringValue) {
            normalized[key] = stringValue
          }
        }
        headers = Object.keys(normalized).length > 0 ? normalized : undefined
      } catch {
        headers = { Host: hostValue }
      }
    } else {
      headers = { Host: hostValue }
    }
  }
  if (!headers && !pathValue && !options.httpupgrade) {
    return undefined
  }
  const wsOpts: WsOptions = {}
  if (pathValue) {
    wsOpts.path = pathValue
  }
  if (headers) {
    wsOpts.headers = headers
  }
  if (options.httpupgrade) {
    wsOpts['v2ray-http-upgrade'] = true
    wsOpts['v2ray-http-upgrade-fast-open'] = true
  }
  return wsOpts
}

export function buildXHttpOptions(
  host: unknown,
  path: unknown,
  mode: unknown,
): XHttpOptions | undefined {
  const hostValue = getTransportHostFirst(host)
  const pathValue = getTransportPathFirst(path)
  const modeValue = normalizeTransportString(firstString(mode))
  if (!hostValue && !pathValue && !modeValue) {
    return undefined
  }
  const xhttpOpts: XHttpOptions = {}
  if (pathValue) {
    xhttpOpts.path = pathValue
  }
  if (hostValue) {
    xhttpOpts.host = hostValue
  }
  if (modeValue) {
    xhttpOpts.mode = modeValue
  }
  return xhttpOpts
}

export function resolveTlsServerName(
  network: NetworkType | undefined,
  options: {
    host?: unknown
    wsOpts?: WsOptions
    httpOpts?: HttpOptions
    h2Opts?: H2Options
    xhttpOpts?: XHttpOptions
  },
): string | undefined {
  switch (network) {
    case 'ws':
      return options.wsOpts?.headers?.Host ?? getTransportHostFirst(options.host)
    case 'http':
      return options.httpOpts?.headers?.Host?.[0] ?? getTransportHostFirst(options.host)
    case 'h2':
      return options.h2Opts?.host ?? getTransportHostFirst(options.host)
    case 'xhttp':
      return options.xhttpOpts?.host ?? getTransportHostFirst(options.host)
    default:
      return getTransportHostFirst(options.host)
  }
}
