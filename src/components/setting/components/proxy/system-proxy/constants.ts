import type { SystemProxyFormValue } from './types'

export const DEFAULT_PROXY_HOST = '127.0.0.1'
export const FALLBACK_HOST_OPTIONS = [DEFAULT_PROXY_HOST, 'localhost']

export const DEFAULT_PAC = `function FindProxyForURL(url, host) {
  return "PROXY %proxy_host%:%mixed-port%; SOCKS5 %proxy_host%:%mixed-port%; DIRECT;";
}`

export const sleep = (ms: number) =>
  new Promise<void>((resolve) => {
    setTimeout(resolve, ms)
  })

export const createSystemProxyFormValue = (
  verge?: IVergeConfig | null,
): SystemProxyFormValue => ({
  guard: verge?.enable_proxy_guard ?? false,
  enable_bypass_check: verge?.enable_bypass_check ?? true,
  bypass: verge?.system_proxy_bypass ?? '',
  duration: verge?.proxy_guard_duration ?? 10,
  use_default: verge?.use_default_bypass ?? true,
  pac: verge?.proxy_auto_config ?? false,
  pac_content: verge?.pac_file_content ?? DEFAULT_PAC,
  proxy_host: verge?.proxy_host ?? DEFAULT_PROXY_HOST,
})
