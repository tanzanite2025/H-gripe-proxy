import { URI_AnyTLS } from './anytls'
import { normalizeUriAndGetScheme } from './helpers'
import { URI_HTTP } from './http'
import { URI_Hysteria } from './hysteria'
import { URI_Hysteria2 } from './hysteria2'
import { URI_Mieru } from './mieru'
import { URI_Snell } from './snell'
import { URI_SOCKS } from './socks'
import { URI_SS } from './ss'
import { URI_SSH } from './ssh'
import { URI_SSR } from './ssr'
import { URI_Sudoku } from './sudoku'
import { URI_Trojan } from './trojan'
import { URI_TrojanGo } from './trojan-go'
import { URI_TUIC } from './tuic'
import { URI_VLESS } from './vless'
import { URI_VMESS } from './vmess'
import { URI_Wireguard } from './wireguard'

type UriParser = (uri: string) => IProxyConfig

const URI_PARSERS: Record<string, UriParser> = {
  ss: URI_SS,
  ssr: URI_SSR,
  ssh: URI_SSH,
  mieru: URI_Mieru,
  mierus: URI_Mieru,
  snell: URI_Snell,
  sudoku: URI_Sudoku,
  vmess: URI_VMESS,
  vless: URI_VLESS,
  trojan: URI_Trojan,
  'trojan-go': URI_TrojanGo,
  anytls: URI_AnyTLS,
  hysteria2: URI_Hysteria2,
  hy2: URI_Hysteria2,
  hysteria: URI_Hysteria,
  hy: URI_Hysteria,
  tuic: URI_TUIC,
  wireguard: URI_Wireguard,
  wg: URI_Wireguard,
  http: URI_HTTP,
  https: URI_HTTP,
  socks5: URI_SOCKS,
  socks: URI_SOCKS,
}

export default function parseUri(uri: string): IProxyConfig {
  const { uri: normalized, scheme } = normalizeUriAndGetScheme(uri)
  const parser = URI_PARSERS[scheme]
  if (!parser) {
    throw new Error(`Unknown uri type: ${scheme}`)
  }
  return parser(normalized)
}
