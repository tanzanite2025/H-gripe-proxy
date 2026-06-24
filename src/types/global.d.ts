type Platform =
  | 'aix'
  | 'android'
  | 'darwin'
  | 'freebsd'
  | 'haiku'
  | 'linux'
  | 'openbsd'
  | 'sunos'
  | 'win32'
  | 'cygwin'
  | 'netbsd'

/**
 * defines in `vite.config.ts`
 */
declare const OS_PLATFORM: Platform

type ValidationOutcome =
  | { status: 'valid' | 'busy' }
  | { status: 'invalid'; kind: string; message: string }
  | { status: 'skipped'; reason: string }

/**
 * Some interface for clash api
 */
interface IConfigData {
  port: number
  mode: 'rule' | 'global'
  ipv6: boolean
  'socket-port': number
  'allow-lan': boolean
  'log-level': string
  'mixed-port': number
  'redir-port': number
  'socks-port': number
  'tproxy-port': number
  'external-controller': string
  'external-controller-cors': {
    'allow-private-network': boolean
    'allow-origins': string[]
  }
  secret: string
  'unified-delay': boolean
  'find-process-mode': 'always' | 'strict' | 'off'
  tun: {
    stack: string
    device: string
    'auto-route': boolean
    'auto-redirect'?: boolean
    'auto-detect-interface': boolean
    'dns-hijack': string[]
    'route-exclude-address'?: string[]
    'strict-route': boolean
    mtu: number
  }
  dns?: {
    enable?: boolean
    listen?: string
    'enhanced-mode'?: 'fake-ip' | 'redir-host'
    'fake-ip-range'?: string
    'fake-ip-filter'?: string[]
    'fake-ip-filter-mode'?: 'blacklist' | 'whitelist'
    'prefer-h3'?: boolean
    'respect-rules'?: boolean
    nameserver?: string[]
    fallback?: string[]
    'default-nameserver'?: string[]
    'proxy-server-nameserver'?: string[]
    'direct-nameserver'?: string[]
    'direct-nameserver-follow-policy'?: boolean
    'nameserver-policy'?: Record<string, any>
    'use-hosts'?: boolean
    'use-system-hosts'?: boolean
    'fallback-filter'?: {
      geoip?: boolean
      'geoip-code'?: string
      ipcidr?: string[]
      domain?: string[]
    }
  }
  tunnels?: {
    network: string[]
    address: string
    target: string
  }[]
  'proxy-groups'?: import('@/types/proxy').IProxyGroupItem[]
}


interface IRuleProviderItem {
  name: string
  behavior: string
  format: string
  ruleCount: number
  type: string
  updatedAt: string
  vehicleType: string
}

interface ITrafficItem {
  up: number
  down: number
  up_rate?: number
  down_rate?: number
  last_updated?: number
}

interface IFormattedTrafficData {
  up_rate_formatted: string
  down_rate_formatted: string
  total_up_formatted: string
  total_down_formatted: string
  is_fresh: boolean
}

interface IFormattedMemoryData {
  inuse_formatted: string
  oslimit_formatted: string
  usage_percent: number
  is_fresh: boolean
}

// 增强的类型安全接口定义，确保所有字段必需
interface ISystemMonitorOverview {
  traffic: {
    raw: {
      up: number
      down: number
      up_rate: number
      down_rate: number
    }
    formatted: {
      up_rate: string
      down_rate: string
      total_up: string
      total_down: string
    }
    is_fresh: boolean
  }
  memory: {
    raw: {
      inuse: number
      oslimit: number
      usage_percent: number
    }
    formatted: {
      inuse: string
      oslimit: string
      usage_percent: number
    }
    is_fresh: boolean
  }
  overall_status: 'active' | 'inactive' | 'error' | 'unknown' | 'healthy'
}

// 类型安全的数据验证器
interface ISystemMonitorOverviewValidator {
  validate(data: any): data is ISystemMonitorOverview
  sanitize(data: any): ISystemMonitorOverview
}

interface ILogItem {
  type: string
  time?: string
  payload: string
}

type LogLevel = import('tauri-plugin-mihomo-api').LogLevel
type LogFilter = 'all' | 'debug' | 'info' | 'warn' | 'err'
type LogOrder = 'asc' | 'desc'

interface IClashLog {
  enable: boolean
  logLevel: LogLevel
  logFilter: LogFilter
  logOrder: LogOrder
}

interface IConnectionsItem {
  id: string
  metadata: {
    network: string
    type: string
    host: string
    sourceIP: string
    sourcePort: string
    destinationPort: string
    destinationIP?: string
    remoteDestination?: string
    process?: string
    processPath?: string
  }
  upload: number
  download: number
  start: string
  chains: string[]
  rule: string
  rulePayload: string
  curUpload?: number // upload speed, calculate at runtime
  curDownload?: number // download speed, calculate at runtime
}

interface IConnections {
  downloadTotal: number
  uploadTotal: number
  connections: IConnectionsItem[]
}

interface IConnectionSetting {
  layout: 'table' | 'list'
}

/**
 * Some interface for command
 */

interface IClashInfo {
  // status: string;
  mixed_port?: number // clash mixed port
  socks_port?: number // clash socks port
  redir_port?: number // clash redir port
  tproxy_port?: number // clash tproxy port
  port?: number // clash http port
  server?: string // external-controller
  secret?: string
}

interface IProfileItem {
  uid: string
  type?: 'local' | 'remote' | 'merge' | 'script' | 'rules' | 'proxies' | 'groups'
  name?: string
  desc?: string
  file?: string
  url?: string
  updated?: number
  selected?: {
    name?: string
    now?: string
  }[]
  extra?: {
    upload: number
    download: number
    total: number
    expire: number
  }
  option?: IProfileOption
  home?: string
}

interface IProfileOption {
  user_agent?: string
  with_proxy?: boolean
  self_proxy?: boolean
  update_interval?: number
  timeout_seconds?: number
  danger_accept_invalid_certs?: boolean
  allow_auto_update?: boolean
  merge?: string
  script?: string
  proxies?: string
  groups?: string
  rules?: string
}

interface IProfilesConfig {
  current?: string
  items?: IProfileItem[]
}

interface IProfilesView extends IProfilesConfig {
  currentPrimaryUid?: string
  primaryItems?: IProfileItem[]
  auxiliaryItems?: IProfileItem[]
}

interface IVergeTestItem {
  uid: string
  name?: string
  icon?: string
  url: string
}
interface IAddress {
  V4?: {
    ip: string
    broadcast?: string
    netmask?: string
  }
  V6?: {
    ip: string
    broadcast?: string
    netmask?: string
  }
}
interface INetworkInterface {
  name: string
  addr: IAddress[]
  mac_addr?: string
  index: number
}

interface ISeqProfileConfig {
  prepend: []
  append: []
  delete: []
}

interface IProxyGroupConfig {
  name: string
  type: 'select' | 'url-test' | 'fallback' | 'load-balance' | 'relay'
  proxies?: string[]
  use?: string[]
  url?: string
  interval?: number
  lazy?: boolean
  timeout?: number
  'max-failed-times'?: number
  'disable-udp'?: boolean
  'interface-name': string
  'routing-mark'?: number
  'include-all'?: boolean
  'include-all-proxies'?: boolean
  'include-all-providers'?: boolean
  filter?: string
  'exclude-filter'?: string
  'exclude-type'?: string
  'expected-status'?: string
  hidden?: boolean
  icon?: string
}

interface WsOptions {
  path?: string
  headers?: {
    [key: string]: string
  }
  'max-early-data'?: number
  'early-data-header-name'?: string
  'v2ray-http-upgrade'?: boolean
  'v2ray-http-upgrade-fast-open'?: boolean
}

interface HttpOptions {
  method?: string
  path?: string[]
  headers?: {
    [key: string]: string[]
  }
}

interface H2Options {
  path?: string
  host?: string
}

interface GrpcOptions {
  'grpc-service-name'?: string
}

interface XHttpOptions {
  path?: string
  host?: string
  mode?: string
  headers?: {
    [key: string]: string
  }
  'no-grpc-header'?: boolean
}

interface RealityOptions {
  'public-key'?: string
  'short-id'?: string
}
interface EchOptions {
  enable?: boolean
  config?: string
  'query-server-name'?: string
}
interface AntiDpiOptions {
  enabled?: boolean
  'padding-mode'?: 'random' | 'size_uniform' | 'none' | string
  'min-padding'?: number
  'max-padding'?: number
  'jitter-ms'?: number
  'burst-before'?: number
  'dummy-traffic'?: boolean
}
interface TrojanPaddingOptions {
  enabled?: boolean
  'min-padding'?: number
  'max-padding'?: number
  'jitter-min'?: number
  'jitter-max'?: number
  'burst-size'?: number
}
interface TrojanBehaviorOptions {
  enabled?: boolean
  'session-simulation'?: boolean
  'idle-timeout-sec'?: number
  'heartbeat-interval-sec'?: number
  'traffic-normalization'?: boolean
  'target-packet-per-sec'?: number
  'target-bytes-per-sec'?: number
  'packet-size-normalization'?: boolean
  'min-packet-size'?: number
  'max-packet-size'?: number
  'adaptive-timing'?: boolean
}
interface TrojanPlusOptions {
  enabled?: boolean
  'mux-enabled'?: boolean
  behavior?: TrojanBehaviorOptions
}
interface Hysteria2RealmOptions {
  enable?: boolean
  'server-url'?: string
  token?: string
  'realm-id'?: string
  'stun-servers'?: string[]
  sni?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  alpn?: string[]
}
type ClientFingerprint =
  | 'chrome'
  | 'firefox'
  | 'safari'
  | 'iOS'
  | 'android'
  | 'edge'
  | '360'
  | 'qq'
  | 'random'
type NetworkType = 'ws' | 'http' | 'h2' | 'grpc' | 'xhttp' | 'tcp'
type CipherType =
  | 'none'
  | 'auto'
  | 'dummy'
  | 'aes-128-gcm'
  | 'aes-192-gcm'
  | 'aes-256-gcm'
  | 'lea-128-gcm'
  | 'lea-192-gcm'
  | 'lea-256-gcm'
  | 'aes-128-gcm-siv'
  | 'aes-256-gcm-siv'
  | '2022-blake3-aes-128-gcm'
  | '2022-blake3-aes-256-gcm'
  | 'aes-128-cfb'
  | 'aes-192-cfb'
  | 'aes-256-cfb'
  | 'aes-128-ctr'
  | 'aes-192-ctr'
  | 'aes-256-ctr'
  | 'chacha20'
  | 'chacha20-ietf'
  | 'chacha20-ietf-poly1305'
  | '2022-blake3-chacha20-poly1305'
  | 'rabbit128-poly1305'
  | 'xchacha20-ietf-poly1305'
  | 'xchacha20'
  | 'aegis-128l'
  | 'aegis-256'
  | 'aez-384'
  | 'deoxys-ii-256-128'
  | 'rc4-md5'
type MieruTransport = 'TCP' | 'UDP'
type MieruMultiplexing =
  | 'MULTIPLEXING_OFF'
  | 'MULTIPLEXING_LOW'
  | 'MULTIPLEXING_MIDDLE'
  | 'MULTIPLEXING_HIGH'
type MieruHandshakeMode =
  | 'HANDSHAKE_DEFAULT'
  | 'HANDSHAKE_STANDARD'
  | 'HANDSHAKE_NO_WAIT'
  | string
type SudokuAeadMethod = 'chacha20-poly1305' | 'aes-128-gcm' | 'none'
type SudokuTableType =
  | 'prefer_ascii'
  | 'prefer_entropy'
  | 'up_ascii_down_entropy'
  | 'up_entropy_down_ascii'
type SudokuHttpMaskMode = 'legacy' | 'stream' | 'poll' | 'auto' | 'ws'
type SudokuHttpMaskMultiplex = 'off' | 'auto' | 'on'
// base
interface IProxyBaseConfig {
  tfo?: boolean
  mptcp?: boolean
  'interface-name'?: string
  'routing-mark'?: number
  'ip-version'?: 'dual' | 'ipv4' | 'ipv6' | 'ipv4-prefer' | 'ipv6-prefer'
  'dialer-proxy'?: string
}
// direct
interface IProxyDirectConfig extends IProxyBaseConfig {
  name: string
  type: 'direct'
}
// dns
interface IProxyDnsConfig extends IProxyBaseConfig {
  name: string
  type: 'dns'
}
// http
interface IProxyHttpConfig extends IProxyBaseConfig {
  name: string
  type: 'http'
  server?: string
  port?: number
  username?: string
  password?: string
  tls?: boolean
  sni?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  headers?: {
    [key: string]: string
  }
}
// socks5
interface IProxySocks5Config extends IProxyBaseConfig {
  name: string
  type: 'socks5'
  server?: string
  port?: number
  username?: string
  password?: string
  tls?: boolean
  udp?: boolean
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
}
// ssh
interface IProxySshConfig extends IProxyBaseConfig {
  name: string
  type: 'ssh'
  server?: string
  port?: number
  username?: string
  password?: string
  'private-key'?: string
  'private-key-passphrase'?: string
  'host-key'?: string[]
  'host-key-algorithms'?: string[]
}
// trojan
interface IProxyTrojanConfig extends IProxyBaseConfig {
  name: string
  type: 'trojan'
  server?: string
  port?: number
  password?: string
  alpn?: string[]
  sni?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  udp?: boolean
  network?: NetworkType
  'ech-opts'?: EchOptions
  'reality-opts'?: RealityOptions
  'grpc-opts'?: GrpcOptions
  'ws-opts'?: WsOptions
  'ss-opts'?: {
    enabled?: boolean
    method?: string
    password?: string
  }
  'client-fingerprint'?: ClientFingerprint
  'padding-opts'?: TrojanPaddingOptions
  'plus-opts'?: TrojanPlusOptions
}
// anytls
interface IProxyAnyTLSConfig extends IProxyBaseConfig {
  name: string
  type: 'anytls'
  server?: string
  port?: number
  password?: string
  alpn?: string[]
  sni?: string
  'client-fingerprint'?: ClientFingerprint
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  'ech-opts'?: {
    enable?: boolean
    config?: string
  }
  udp?: boolean
  'idle-session-check-interval'?: number
  'idle-session-timeout'?: number
  'min-idle-session'?: number
}
// tuic
interface IProxyTuicConfig extends IProxyBaseConfig {
  name: string
  type: 'tuic'
  server?: string
  port?: number
  token?: string
  uuid?: string
  password?: string
  ip?: string
  'heartbeat-interval'?: number
  alpn?: string[]
  'reduce-rtt'?: boolean
  'request-timeout'?: number
  'udp-relay-mode'?: string
  'congestion-controller'?: string
  'disable-sni'?: boolean
  'max-udp-relay-packet-size'?: number
  'fast-open'?: boolean
  'max-open-streams'?: number
  cwnd?: number
  'bbr-profile'?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  ca?: string
  'ca-str'?: string
  'recv-window-conn'?: number
  'recv-window'?: number
  'disable-mtu-discovery'?: boolean
  'max-datagram-frame-size'?: number
  sni?: string
  'ech-opts'?: EchOptions
  'udp-over-stream'?: boolean
  'udp-over-stream-version'?: number
}
// mieru
interface IProxyMieruConfig extends IProxyBaseConfig {
  name: string
  type: 'mieru'
  server?: string
  port?: number
  'port-range'?: string
  transport?: MieruTransport
  udp?: boolean
  username?: string
  password?: string
  multiplexing?: MieruMultiplexing
  'handshake-mode'?: MieruHandshakeMode
  'traffic-pattern'?: string
}
// masque
interface IProxyMasqueConfig extends IProxyBaseConfig {
  name: string
  type: 'masque'
  server?: string
  port?: number
  'private-key'?: string
  'public-key'?: string
  ip?: string
  ipv6?: string
  uri?: string
  sni?: string
  mtu?: number
  udp?: boolean
  'skip-cert-verify'?: boolean
  network?: string
  'congestion-controller'?: string
  cwnd?: number
  'bbr-profile'?: string
  'remote-dns-resolve'?: boolean
  dns?: string[]
}
// gost relay
interface IProxyGostRelayConfig extends IProxyBaseConfig {
  name: string
  type: 'gost-relay'
  server?: string
  port?: number
  forward?: boolean
  udp?: boolean
  tls?: boolean
  mux?: boolean
  sni?: string
  username?: string
  password?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  'client-fingerprint'?: ClientFingerprint
}
// trust tunnel
interface IProxyTrustTunnelConfig extends IProxyBaseConfig {
  name: string
  type: 'trusttunnel'
  server?: string
  port?: number
  username?: string
  password?: string
  alpn?: string[]
  sni?: string
  'ech-opts'?: {
    enable?: boolean
    config?: string
  }
  'client-fingerprint'?: ClientFingerprint
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  udp?: boolean
  'health-check'?: boolean
  quic?: boolean
  'congestion-controller'?: string
  cwnd?: number
  'bbr-profile'?: string
  'max-connections'?: number
  'min-streams'?: number
  'max-streams'?: number
}
// openvpn
interface IProxyOpenVPNConfig extends IProxyBaseConfig {
  name: string
  type: 'openvpn'
  server?: string
  port?: number
  proto?: string
  dev?: string
  cipher?: string
  auth?: string
  'comp-lzo'?: string
  ca?: string
  cert?: string
  key?: string
  'tls-crypt'?: string
  username?: string
  password?: string
  mtu?: number
  udp?: boolean
  'remote-dns-resolve'?: boolean
  dns?: string[]
}
// tailscale
interface IProxyTailscaleConfig extends IProxyBaseConfig {
  name: string
  type: 'tailscale'
  hostname?: string
  'auth-key'?: string
  'control-url'?: string
  'state-dir'?: string
  ephemeral?: boolean
  udp?: boolean
  'accept-routes'?: boolean
  'exit-node'?: string
  'exit-node-allow-lan-access'?: boolean
}
// reject
interface IProxyRejectConfig extends IProxyBaseConfig {
  name: string
  type: 'reject'
}
// vless
interface IProxyVlessConfig extends IProxyBaseConfig {
  name: string
  type: 'vless'
  server?: string
  port?: number
  uuid?: string
  flow?: string
  tls?: boolean
  alpn?: string[]
  udp?: boolean
  'packet-addr'?: boolean
  xudp?: boolean
  'packet-encoding'?: string
  encryption?: string
  network?: NetworkType
  'ech-opts'?: EchOptions
  'reality-opts'?: RealityOptions
  'http-opts'?: HttpOptions
  'h2-opts'?: H2Options
  'grpc-opts'?: GrpcOptions
  'xhttp-opts'?: XHttpOptions
  'ws-opts'?: WsOptions
  'ws-path'?: string
  'ws-headers'?: {
    [key: string]: string
  }
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  servername?: string
  'client-fingerprint'?: ClientFingerprint
  'anti-dpi-opts'?: AntiDpiOptions
  smux?: boolean
}
// vmess
interface IProxyVmessConfig extends IProxyBaseConfig {
  name: string
  type: 'vmess'
  server?: string
  port?: number
  uuid?: string
  alterId?: number
  cipher?: CipherType
  udp?: boolean
  network?: NetworkType
  tls?: boolean
  alpn?: string[]
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  servername?: string
  'ech-opts'?: EchOptions
  'reality-opts'?: RealityOptions
  'http-opts'?: HttpOptions
  'h2-opts'?: H2Options
  'grpc-opts'?: GrpcOptions
  'ws-opts'?: WsOptions
  'packet-addr'?: boolean
  xudp?: boolean
  'packet-encoding'?: string
  'global-padding'?: boolean
  'authenticated-length'?: boolean
  'client-fingerprint'?: ClientFingerprint
  'anti-dpi-opts'?: AntiDpiOptions
  smux?: boolean
}
interface WireGuardPeerOptions {
  server?: string
  port?: number
  'public-key'?: string
  'pre-shared-key'?: string
  reserved?: number[]
  'allowed-ips'?: string[]
}
// wireguard
interface IProxyWireguardConfig extends IProxyBaseConfig, WireGuardPeerOptions {
  name: string
  type: 'wireguard'
  ip?: string
  ipv6?: string
  'private-key'?: string
  workers?: number
  mtu?: number
  udp?: boolean
  'persistent-keepalive'?: number
  peers?: WireGuardPeerOptions[]
  'remote-dns-resolve'?: boolean
  dns?: string[]
  'refresh-server-ip-interval'?: number
}
// hysteria
interface IProxyHysteriaConfig extends IProxyBaseConfig {
  name: string
  type: 'hysteria'
  server?: string
  port?: number
  ports?: string
  protocol?: string
  'obfs-protocol'?: string
  up?: string
  'up-speed'?: number
  down?: string
  'down-speed'?: number
  auth?: string
  'auth-str'?: string
  obfs?: string
  sni?: string
  'ech-opts'?: EchOptions
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  alpn?: string[]
  ca?: string
  'ca-str'?: string
  'recv-window-conn'?: number
  'recv-window'?: number
  'disable-mtu-discovery'?: boolean
  'fast-open'?: boolean
  'hop-interval'?: number
}
// hysteria2
interface IProxyHysteria2Config extends IProxyBaseConfig {
  name: string
  type: 'hysteria2'
  server?: string
  port?: number
  ports?: string
  'hop-interval'?: number
  protocol?: string
  'obfs-protocol'?: string
  up?: string
  down?: string
  password?: string
  obfs?: string
  'obfs-password'?: string
  'obfs-min-packet-size'?: number
  'obfs-max-packet-size'?: number
  sni?: string
  'ech-opts'?: EchOptions
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  alpn?: string[]
  ca?: string
  'ca-str'?: string
  cwnd?: number
  'bbr-profile'?: string
  'udp-mtu'?: number
  'adaptive-bw'?: boolean
  'realm-opts'?: Hysteria2RealmOptions
  'initial-stream-receive-window'?: number
  'max-stream-receive-window'?: number
  'initial-connection-receive-window'?: number
  'max-connection-receive-window'?: number
}
// shadowsocks
interface IProxyShadowsocksConfig extends IProxyBaseConfig {
  name: string
  type: 'ss'
  server?: string
  port?: number
  password?: string
  cipher?: CipherType
  udp?: boolean
  plugin?: 'obfs' | 'v2ray-plugin' | 'shadow-tls' | 'restls'
  'plugin-opts'?: {
    mode?: string
    host?: string
    password?: string
    path?: string
    tls?: string
    fingerprint?: string
    headers?: {
      [key: string]: string
    }
    'skip-cert-verify'?: boolean
    version?: number
    mux?: boolean
    'v2ray-http-upgrade'?: boolean
    'v2ray-http-upgrade-fast-open'?: boolean
    'version-hint'?: string
    'restls-script'?: string
  }
  'udp-over-tcp'?: boolean
  'udp-over-tcp-version'?: number
  'client-fingerprint'?: ClientFingerprint
  smux?: boolean
}
// sudoku
interface IProxySudokuConfig extends IProxyBaseConfig {
  name: string
  type: 'sudoku'
  server?: string
  port?: number
  key?: string
  'aead-method'?: SudokuAeadMethod
  'padding-min'?: number
  'padding-max'?: number
  'table-type'?: SudokuTableType
  'enable-pure-downlink'?: boolean
  'http-mask'?: boolean
  'http-mask-mode'?: SudokuHttpMaskMode
  'http-mask-tls'?: boolean
  'http-mask-host'?: string
  'http-mask-multiplex'?: SudokuHttpMaskMultiplex
  httpmask?: {
    disable?: boolean
    mode?: SudokuHttpMaskMode
    tls?: boolean
    host?: string
    'path-root'?: string
    multiplex?: SudokuHttpMaskMultiplex
  }
  'custom-table'?: string
  'custom-tables'?: string[]
}
// shadowsocksR
interface IProxyshadowsocksRConfig extends IProxyBaseConfig {
  name: string
  type: 'ssr'
  server?: string
  port?: number
  password?: string
  cipher?: CipherType
  obfs?: string
  'obfs-param'?: string
  protocol?: string
  'protocol-param'?: string
  udp?: boolean
}
// sing-mux
interface IProxySmuxConfig {
  smux?: {
    enabled?: boolean
    protocol?: 'smux' | 'yamux' | 'h2mux'
    'max-connections'?: number
    'min-streams'?: number
    'max-streams'?: number
    padding?: boolean
    statistic?: boolean
    'only-tcp'?: boolean
    'brutal-opts'?: {
      enabled?: boolean
      up?: string
      down?: string
    }
  }
}
// snell
interface IProxySnellConfig extends IProxyBaseConfig {
  name: string
  type: 'snell'
  server?: string
  port?: number
  psk?: string
  udp?: boolean
  version?: number
  reuse?: boolean
  'obfs-opts'?: {
    mode?: 'http' | 'tls'
    host?: string
  }
}
interface IProxyConfig
  extends IProxyBaseConfig,
    IProxyDirectConfig,
    IProxyDnsConfig,
    IProxyHttpConfig,
    IProxySocks5Config,
    IProxySshConfig,
    IProxyTrojanConfig,
    IProxyAnyTLSConfig,
    IProxyTuicConfig,
    IProxyMieruConfig,
    IProxyMasqueConfig,
    IProxyGostRelayConfig,
    IProxyTrustTunnelConfig,
    IProxyOpenVPNConfig,
    IProxyTailscaleConfig,
    IProxyRejectConfig,
    IProxyVlessConfig,
    IProxyVmessConfig,
    IProxyWireguardConfig,
    IProxyHysteriaConfig,
    IProxyHysteria2Config,
    IProxyShadowsocksConfig,
    IProxySudokuConfig,
    IProxyshadowsocksRConfig,
    IProxySmuxConfig,
    IProxySnellConfig {
  type:
    | 'ss'
    | 'ssr'
    | 'direct'
    | 'dns'
    | 'snell'
    | 'http'
    | 'trojan'
    | 'anytls'
    | 'hysteria'
    | 'hysteria2'
    | 'tuic'
    | 'wireguard'
    | 'ssh'
    | 'socks5'
    | 'masque'
    | 'gost-relay'
    | 'trusttunnel'
    | 'openvpn'
    | 'tailscale'
    | 'reject'
    | 'vmess'
    | 'vless'
    | 'mieru'
    | 'sudoku'
}

interface IVergeConfig {
  app_log_level?: 'trace' | 'debug' | 'info' | 'warn' | 'error' | string
  app_log_max_size?: number // KB
  app_log_max_count?: number
  language?: string
  env_type?: 'bash' | 'cmd' | 'powershell' | 'fish' | string
  startup_script?: string
  start_page?: string
  theme_mode?: 'light' | 'dark' | 'system'
  menu_order?: string[]
  enable_tun_mode?: boolean
  enable_auto_launch?: boolean
  enable_silent_start?: boolean
  enable_system_proxy?: boolean
  enable_global_hotkey?: boolean
  enable_dns_settings?: boolean
  proxy_auto_config?: boolean
  pac_file_content?: string
  proxy_host?: string
  enable_random_port?: boolean
  verge_mixed_port?: number
  verge_socks_port?: number
  verge_redir_port?: number
  verge_tproxy_port?: number
  verge_port?: number
  verge_redir_enabled?: boolean
  verge_tproxy_enabled?: boolean
  verge_socks_enabled?: boolean
  verge_http_enabled?: boolean
  enable_proxy_guard?: boolean
  enable_bypass_check?: boolean
  use_default_bypass?: boolean
  proxy_guard_duration?: number
  system_proxy_bypass?: string
  hotkeys?: string[]
  auto_check_update?: boolean
  default_latency_test?: string
  default_latency_timeout?: number
  auto_log_clean?: 0 | 1 | 2 | 3 | 4
  enable_auto_backup_schedule?: boolean
  auto_backup_interval_hours?: number
  auto_backup_on_change?: boolean
  proxy_layout_column?: number
  test_list?: IVergeTestItem[]
  webdav_url?: string
  webdav_username?: string
  webdav_password?: string
  home_cards?: Record<string, boolean>
  enable_tor_proxy?: boolean
  tor_socks_host?: string
  tor_socks_port?: number
  tor_control_port?: number
  tor_use_bridges?: boolean
  tor_bridges?: string[]
  enable_external_controller?: boolean
}

interface IWebDavFile {
  filename: string
  href: string
  last_modified: string
  content_length: number
  content_type: string
  tag: string
}

interface ILocalBackupFile {
  filename: string
  path: string
  last_modified: string
  content_length: number
}

interface IWebDavConfig {
  url: string
  username: string
  password: string
}

// Traffic monitor types
interface ITrafficDataPoint {
  up: number
  down: number
  timestamp: number
  name: string
}

interface ISamplingConfig {
  rawDataMinutes: number
  compressedDataMinutes: number
  compressionRatio: number
}

interface ISamplerStats {
  rawBufferSize: number
  compressedBufferSize: number
  compressionQueueSize: number
  totalMemoryPoints: number
}

interface ITrafficWorkerInitMessage {
  type: 'init'
  config: ISamplingConfig & {
    snapshotIntervalMs: number
    defaultRangeMinutes: number
  }
}

interface ITrafficWorkerAppendMessage {
  type: 'append'
  payload: {
    up: number
    down: number
    timestamp?: number
  }
}

interface ITrafficWorkerClearMessage {
  type: 'clear'
}

interface ITrafficWorkerSetRangeMessage {
  type: 'setRange'
  minutes: number
}

interface ITrafficWorkerRequestSnapshotMessage {
  type: 'requestSnapshot'
}

type TrafficWorkerRequestMessage =
  | ITrafficWorkerInitMessage
  | ITrafficWorkerAppendMessage
  | ITrafficWorkerClearMessage
  | ITrafficWorkerSetRangeMessage
  | ITrafficWorkerRequestSnapshotMessage

interface ITrafficWorkerSnapshotMessage {
  type: 'snapshot'
  dataPoints: ITrafficDataPoint[]
  availableDataPoints: ITrafficDataPoint[]
  samplerStats: ISamplerStats
  rangeMinutes: number
  lastTimestamp?: number
  reason:
    | 'init'
    | 'interval'
    | 'range-change'
    | 'request'
    | 'append-throttle'
    | 'clear'
}

interface ITrafficWorkerLogMessage {
  type: 'log'
  message: string
}

type TrafficWorkerResponseMessage =
  | ITrafficWorkerSnapshotMessage
  | ITrafficWorkerLogMessage

/** Single rule within a security policy */
interface IPolicyRule {
  ruleType: string
  payload: string
  proxy: string
}

/** A security policy definition */
interface ISecurityPolicy {
  name: string
  enabled: boolean
  description: string
  rules: IPolicyRule[]
}

/** Runtime state of an applied security policy */
interface IAppliedPolicyState {
  name: string
  enabled: boolean
  ruleIndices: number[]
  applied: boolean
}
