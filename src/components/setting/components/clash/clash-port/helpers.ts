import type { ClashPortValues } from './types'

const DEFAULT_MIXED_PORT = 7897
const DEFAULT_SOCKS_PORT = 7898
const DEFAULT_HTTP_PORT = 7899
const DEFAULT_REDIR_PORT = 7895
const DEFAULT_TPROXY_PORT = 7896

export const generateRandomPort = () =>
  Math.floor(Math.random() * (65535 - 1025 + 1)) + 1025

export const createClashPortValues = (
  verge?: IVergeConfig | null,
  clashInfo?: IClashInfo | null,
): ClashPortValues => ({
  mixedPort: verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? DEFAULT_MIXED_PORT,
  socksPort: verge?.verge_socks_port ?? DEFAULT_SOCKS_PORT,
  socksEnabled: verge?.verge_socks_enabled ?? false,
  httpPort: verge?.verge_port ?? DEFAULT_HTTP_PORT,
  httpEnabled: verge?.verge_http_enabled ?? false,
  redirPort: verge?.verge_redir_port ?? DEFAULT_REDIR_PORT,
  redirEnabled: verge?.verge_redir_enabled ?? false,
  tproxyPort: verge?.verge_tproxy_port ?? DEFAULT_TPROXY_PORT,
  tproxyEnabled: verge?.verge_tproxy_enabled ?? false,
})

export const hasDuplicatePorts = (values: ClashPortValues) => {
  const activePorts = [
    values.mixedPort,
    values.socksEnabled ? values.socksPort : -1,
    values.httpEnabled ? values.httpPort : -1,
    values.redirEnabled ? values.redirPort : -1,
    values.tproxyEnabled ? values.tproxyPort : -1,
  ].filter((port) => port !== -1)

  return new Set(activePorts).size !== activePorts.length
}

export const hasOnlyValidPorts = (values: ClashPortValues) => {
  const isValidPort = (port: number) => port >= 1 && port <= 65535

  return [
    values.mixedPort,
    values.socksEnabled ? values.socksPort : 0,
    values.httpEnabled ? values.httpPort : 0,
    values.redirEnabled ? values.redirPort : 0,
    values.tproxyEnabled ? values.tproxyPort : 0,
  ].every((port) => port === 0 || isValidPort(port))
}

export const collectChangedPorts = (
  nextValues: ClashPortValues,
  originalValues: ClashPortValues | null,
) => {
  if (!originalValues) {
    return [
      nextValues.mixedPort,
      nextValues.socksEnabled ? nextValues.socksPort : -1,
      nextValues.httpEnabled ? nextValues.httpPort : -1,
      nextValues.redirEnabled ? nextValues.redirPort : -1,
      nextValues.tproxyEnabled ? nextValues.tproxyPort : -1,
    ].filter((port) => port !== -1)
  }

  const changedPorts: number[] = []

  if (nextValues.mixedPort !== originalValues.mixedPort) {
    changedPorts.push(nextValues.mixedPort)
  }
  if (nextValues.socksEnabled && nextValues.socksPort !== originalValues.socksPort) {
    changedPorts.push(nextValues.socksPort)
  }
  if (nextValues.httpEnabled && nextValues.httpPort !== originalValues.httpPort) {
    changedPorts.push(nextValues.httpPort)
  }
  if (nextValues.redirEnabled && nextValues.redirPort !== originalValues.redirPort) {
    changedPorts.push(nextValues.redirPort)
  }
  if (nextValues.tproxyEnabled && nextValues.tproxyPort !== originalValues.tproxyPort) {
    changedPorts.push(nextValues.tproxyPort)
  }

  return changedPorts
}

export const buildClashPortConfigs = (values: ClashPortValues) => ({
  clashConfig: {
    'mixed-port': values.mixedPort,
    'socks-port': values.socksPort,
    port: values.httpPort,
    'redir-port': values.redirPort,
    'tproxy-port': values.tproxyPort,
  },
  vergeConfig: {
    verge_mixed_port: values.mixedPort,
    verge_socks_port: values.socksPort,
    verge_socks_enabled: values.socksEnabled,
    verge_port: values.httpPort,
    verge_http_enabled: values.httpEnabled,
    verge_redir_port: values.redirPort,
    verge_redir_enabled: values.redirEnabled,
    verge_tproxy_port: values.tproxyPort,
    verge_tproxy_enabled: values.tproxyEnabled,
  },
})
