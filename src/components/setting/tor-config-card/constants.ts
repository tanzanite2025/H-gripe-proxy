export const DEFAULT_TOR_CONFIG = {
  enabled: false,
  socksHost: '127.0.0.1',
  socksPort: 9050,
  controlPort: 9051,
  useBridges: false,
  bridges: [] as string[],
}

export type TorDraftConfig = typeof DEFAULT_TOR_CONFIG

export const TOR_USAGE_INSTRUCTIONS = {
  title: 'Tor 使用说明',
  steps: [
    '下载安装 Tor Browser 或 Tor Expert Bundle。',
    '启动 Tor，确认本地 SOCKS5 服务运行在 127.0.0.1:9050。',
    '在 Clash Verge Optimized 中启用 Tor 代理并填写本地监听地址。',
    '按需要把部分规则或应用流量切到 Tor。',
    '如果所在网络环境有限制，可以开启桥接并填入 bridge 列表。',
  ],
  notes: [
    'Tor 会明显降低速度，通常只适合隐私优先场景。',
    '建议同时配合 DoH 或其他 DNS 防泄漏方案使用。',
    '某些网络环境可能需要桥接节点，否则无法建立线路。',
    '不建议在 Tor 上进行大流量下载或长时间高并发传输。',
    '为提高匿名性，可以定期更换 Tor 电路或重启线路。',
  ],
}

export const buildTorSocksUrl = (host: string, port: number) =>
  `socks5://${host}:${port}`

export const parseBridgeList = (value: string) =>
  value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean)
