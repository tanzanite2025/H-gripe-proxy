import { parseTrojanUri } from './trojan'

export function URI_TrojanGo(line: string): IProxyTrojanConfig {
  return parseTrojanUri(line, 'trojan-go')
}
