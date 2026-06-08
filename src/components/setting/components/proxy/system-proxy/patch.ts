import { DEFAULT_PAC } from './constants'
import type { SystemProxyFormValue } from './types'
import { normalizeProxyHost } from './validation'

interface BuildPatchArgs {
  value: SystemProxyFormValue
  current: Pick<
    IVergeConfig,
    | 'enable_proxy_guard'
    | 'enable_bypass_check'
    | 'proxy_guard_duration'
    | 'system_proxy_bypass'
    | 'proxy_auto_config'
    | 'use_default_bypass'
    | 'pac_file_content'
    | 'proxy_host'
  >
  mixedPort?: number
}

const renderPacContent = (
  pacContent: string | undefined,
  proxyHost: string,
  mixedPort?: number,
) => {
  const source = pacContent || DEFAULT_PAC
  const mixedPortValue = String(mixedPort ?? '')

  return source
    .replace(/%proxy_host%/g, proxyHost)
    .replace(/%mixed-port%/g, mixedPortValue)
}

export function buildSystemProxyPatch({
  value,
  current,
  mixedPort,
}: BuildPatchArgs) {
  const patch: Partial<IVergeConfig> = {}

  if (value.guard !== current.enable_proxy_guard) {
    patch.enable_proxy_guard = value.guard
  }
  if (value.enable_bypass_check !== current.enable_bypass_check) {
    patch.enable_bypass_check = value.enable_bypass_check
  }
  if (value.duration !== current.proxy_guard_duration) {
    patch.proxy_guard_duration = value.duration
  }
  if (value.bypass !== current.system_proxy_bypass) {
    patch.system_proxy_bypass = value.bypass
  }
  if (value.pac !== current.proxy_auto_config) {
    patch.proxy_auto_config = value.pac
  }
  if (value.use_default !== current.use_default_bypass) {
    patch.use_default_bypass = value.use_default
  }

  const nextPacContent = renderPacContent(
    value.pac_content,
    value.proxy_host,
    mixedPort,
  )
  if (nextPacContent !== current.pac_file_content) {
    patch.pac_file_content = nextPacContent
  }

  const nextProxyHost = normalizeProxyHost(value.proxy_host)
  if (nextProxyHost !== current.proxy_host) {
    patch.proxy_host = nextProxyHost
  }

  const needResetProxy =
    value.pac !== current.proxy_auto_config ||
    nextProxyHost !== current.proxy_host ||
    nextPacContent !== current.pac_file_content ||
    value.bypass !== current.system_proxy_bypass ||
    value.use_default !== current.use_default_bypass

  return {
    patch,
    nextPacContent,
    nextProxyHost,
    needResetProxy,
  }
}
