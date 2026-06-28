# learn-gripe 内核协议能力矩阵 & 优先级路线图

> 目的：在一处记录内核数据面「现在能跑什么、缺什么、接下来按什么顺序补」，作为后续排期的决策依据。
> 维度区分两件事：**能解析（parse）** vs **能跑流量（data plane）**。本文只关心后者——「能不能真正转发」。
> 来源：逐行核对 `crates/learn-gripe/src/{outbound,transport,protocols,udp,inbound}`。
> 图例：✅ 已实现并接通　❌ 显式 bail 拒绝（不会静默乱编码）　△ 部分　— 不适用
>
> 最近更新：**v2ray-plugin 非 websocket / mux 已补**——`v2ray-http-upgrade`（复用 httpupgrade transport）、`mux: true`（mux.cool 单子连接帧封装）、`mode: quic`（标准 QUIC + 强制 TLS + 按 server 连接池复用，ALPN 默认 `[h2,http/1.1]`），互通测试见 `tests/shadowsocks_plugin.rs`；SS SIP003 plugin 全部完成（simple-obfs http/伪 TLS、v2ray-plugin ws±tls，PR #449/#450）；**ECH（Encrypted Client Hello）已接线**——内置 RFC 9180 HPKE provider（DHKEM-X25519-HKDF-SHA256 + AES-128/256-GCM、ChaCha20Poly1305），`ech-opts` 现已驱动真实握手。

---

## 0. 先回答一个常见疑问："Shadowsocks 2022" 是不是已经过时了？

**不是，这是命名误解。** "Shadowsocks 2022"（规范代号 **SIP022**）里的 "2022" 是**定稿年份**，不是版本有效期，也没有所谓 "SS 2025/2026" 来取代它。到今天它仍然是 Shadowsocks 的**当前推荐方法**：`shadowsocks-rust`、`sing-box`、`mihomo/clash.meta` 现在新发的 SS 节点默认就是 `2022-blake3-*`。

- 相比 2017 AEAD（`aes-*-gcm` / `chacha20-ietf-poly1305`），2022 用 **BLAKE3** 做会话子密钥派生、PSK 直接用 base64 原始密钥、并加了带时间戳的握手头来抗重放/抗探测。
- 它和主流服务端**一定能互通**——本仓库的 TCP 互通测试用独立实现的 fake server 证明了 BLAKE3 密钥派生 / 头格式 / nonce 全部对得上（见 `crates/learn-gripe/tests/shadowsocks2022_outbound.rs`）。

**真正"新"的前沿**在别处：2026 年抗封锁主力已经往 **QUIC 系（Hysteria2 / TUIC）** 和 **REALITY + VLESS-Vision** 走。SS-2022 属于"稳、常见、低风险"的补齐项；QUIC 系才是大工程、大增量（见 §5）。

---

## 1. 出站协议（`OutboundMode`）

| 协议 | TCP | UDP | 备注 |
|---|---|---|---|
| direct | ✅ | ✅ | UDP 走 OS socket |
| reject | ✅ | — | 阻断 |
| socks5（上游代理） | ✅ | ❌ | 仅 CONNECT；上游 SOCKS5 无 UDP relay |
| http（上游代理） | ✅ | ❌ | HTTP `CONNECT` 隧道；可选 `Proxy-Authorization: Basic`（`username`/`password`）+ 可选 `tls: true`（HTTPS 代理，复用共享 TLS 层：SNI/ALPN/`skip-cert-verify`/`client-fingerprint`）；server 保留为域名按需解析；无 UDP relay（互通测试见 `tests/http_outbound.rs`） |
| shadowsocks (ss) | ✅ | ✅ | cipher 限制见 §3；2017 系与 2022 系均 TCP+UDP；SIP003 plugin: simple-obfs(http/tls)、v2ray-plugin(ws±tls / http-upgrade / mux / quic) |
| trojan | ✅ | ✅ | 经 `build_layers` 全传输/安全 |
| vmess | ✅ | ✅ | 仅 alterId 0 (AEAD)；cipher auto / aes-128-gcm / chacha20-poly1305 |
| vless | ✅ | ✅ | 支持 Vision（仅 raw TCP）；encryption 须 none |
| tuic | ✅ | ✅ | TUIC v5（QUIC，quinn 栈）；TLS 密钥导出鉴权 + bidi 流 TCP relay；UDP 两种 `udp-relay-mode`：`native` 走 `Packet` 命令的 QUIC datagram（分片/重组），`quic`(uni-stream) 每个 datagram 走独立单向 QUIC 流（可靠、无 datagram-MTU 上限，单 `Packet` 整包，见 `tests/tuic_quic_udp_outbound.rs`）；**连接池**：无 `reduce-rtt` 时按 server 指纹（server/port/uuid/password/sni/alpn/cc）进程级弱引用复用一条鉴权 QUIC 连接，并发 relay 各开自己的流、共用一次握手+鉴权，最后一个 relay 结束即关闭不空转（见 `tests/tuic_pool.rs`）；`reduce-rtt` 0-RTT（`Connect` 走早期数据、鉴权等握手完成）则每次新拨以便早期数据 |
| hysteria2 | ✅ | ✅ | QUIC（quinn 栈）；HTTP/3 `POST /auth` 鉴权（`h3`/`h3-quinn`）+ 裸 QUIC 流 `TCPRequest(0x401)` TCP relay；UDP 走 QUIC datagram（`Hysteria-UDP` 协商 + 分片/重组）；Salamander obfs / 端口跳跃；`reduce-rtt` 0-RTT（鉴权 + `TCPRequest` 走早期数据） |
| anytls | ✅ | ✅ | TLS 之上的会话层（`build_layers` 全传输/安全）；`SHA256(password)` 认证 + `cmdSettings`/`cmdSYN`/`cmdPSH(SocksAddr)` 帧**并发多路复用**：每个 session 一条 TLS 连接拆成后台 reader/writer 两个任务（`tokio::io::split`，读写互不阻塞，规避双向背压死锁）：reader 读帧按 stream id demux `cmdPSH`/`cmdFIN` 到各逻辑流的有界 channel，writer 处理各流写/开流/关流命令并统一过 per-session padding shaper 写线（心跳响应由 reader 转交 writer 发送），逻辑流 `MuxStream` 退化成 session 上的一个流；按 `server:port` 进程级**会话注册表复用**：新连接优先在已有 session 的空闲 stream slot 上开新流（并发多路复用 / 空闲复用，发新 `cmdSYN` stream id 递增，省掉 TLS 握手 + 认证），slot 满（`MAX_STREAMS_PER_SESSION`）或 session 损坏时才新建 TLS；session 无活动流时按空闲 TTL 回收，损坏（传输断 / `cmdAlert`）即逐出不复用；处理 `cmdSYNACK`/`cmdFIN`/`cmdAlert`/`cmdHeartRequest`；UDP 走 udp-over-tcp v2（开到 `sp.v2.udp-over-tcp.arpa`，connect 模式 `len(2)|payload` 逐包，逐目的一条流）；padding scheme 流量整形已实现（解析默认 `padding-md5` DSL，按 `writeConn` 逐"包"分包并插 `cmdWaste` 填充，padding0 随认证头发送，到 `stop` 后停用）；收到 `cmdUpdatePaddingScheme` 时按 `server:port` 进程级存储（校验 md5 不同才替换），当前连接保持原 scheme，后续到该 server 的新连接广告并按下发的 scheme 整形（对齐上游 anytls-go per-server `Client` 语义） |
| snell | ✅ | ✅ | Shadowsocks-AEAD 分块帧（16B salt + `AEAD(len)|AEAD(payload)`，12B LE 计数 nonce），会话子密钥用 **Argon2id**（`t=3,m=8KiB,p=1,32` 截断）而非 HKDF；cipher 按版本：v1 ChaCha20-Poly1305 / v2·v3 AES-128-GCM；请求头 `proto|command|clientID-len(0)|host|port`，首回包字节为命令响应（`Tunnel`/`Error`）；v2 走 `CommandConnectV2`；独立 fake Snell server 互通测试 `tests/snell_outbound.rs`（v1/v3、多块大包、Routed）。**UDP（`CommandUDP`，仅 v3）已完成**：UDP-over-TCP 跑同一 shadowaead 流，握手头 `proto|CommandUDP(6)|clientID-len(0)`（无 host/port），逐包一个 AEAD chunk（1 chunk = 1 datagram），明文 c→s `UDPForward(0x01)|addr|payload` / s→c `addr|payload`，addr 为 snell 专用格式（域名 `len|host|port`、IP `00|family(4·6)|addr|port`，回程去 `00` 前缀）；逐目的一条 `SnellUdp` 关联（`send`/`recv`，split 读写各自 cipher+nonce）；独立 fake Snell UDP server 互通测试 `tests/snell_udp.rs`（IPv4/IPv6/域名、多包、大包、Routed）。**`obfs-opts`(http·tls simple-obfs) 已完成**：复用现有 `transport::simple_obfs::connect_http/connect_tls`，挂在 AEAD 之下（`connect_transport` 包裹 socket），对 TCP 与 UDP-over-TCP 同样适用；`SnellObfs`(http `host`/`path` / tls `host`) 解析 `obfs-opts`，未知 mode 拒绝；独立 fake obfs server 互通测试 `tests/snell_obfs.rs`（http·tls × TCP·UDP、大包）。**会话复用 / 连接池（v2 始终 / v4·v5 配 `reuse`）已完成**：v2 用 `CommandConnectV2` + HalfClose（写零长 AEAD chunk = 只发 `AEAD(0x0000)` 长度块、无 payload 块，对端解出长度 0 视作逻辑 EOF，对齐上游 `writeZeroChunk`/`ErrZeroChunk`），在同一条 TCP 上**顺序复用**（shadowaead 的 salt/cipher/计数 nonce 跨逻辑流连续推进，每个新请求重写 header、读一个新的命令响应字节）；按 `server:port`（含 version/psk/obfs）进程级**连接池**（仿 AnyTLS 注册表，空闲 TTL 15s + 上限 10，过期/超量逐出关闭）：`connect` 对 v2 优先取池中空闲会话（省去 TCP+salt 握手），否则新拨；`SnellStream::poll_shutdown` 对可复用流发半关闭零长 chunk 而不关 TCP，`Drop` 在双向零长 chunk 干净交换且无残留时归还池（否则关连接），故 `copy_bidirectional` 双向 EOF 后天然只回收空闲连接；v1/v3 仍一次性新拨。独立 fake 复用 server 互通测试 `tests/snell_reuse.rs`（多个顺序流共用一条 TCP、复用大包）+ 单元测试（顺序复用、空闲 TTL 逐出）。**v4/v5 连接类型已完成**：v4/v5 改用与 shadowaead 不同的**帧流**（`SnellV4Stream`），仍是 16B salt + Argon2id 子密钥 + AES-128-GCM + 12B 计数 nonce，但每帧 = `AEAD(7B 头) | [padding] | AEAD(payload)`，头 = `0x04|0|0|padLen(BE16)|payLen(BE16)`；**首帧**前置 16B salt + 初始随机 padding（长度 `[0x100,0x200)`，与 payload 密文按偶数字节 `swapPadding` 交织、纯反检测），`payLen==0` 帧即逻辑 EOF（`ErrZeroChunk`）；请求头/命令响应字节逻辑同 v1-v3，命令用 `CommandConnect`；**v5 在线路上即 v4**（上游把 v5 配置当 v4 客户端拨，`from_proxy` 把 `version:5` 归一为 4）。**v4/v5 UDP 已完成**：v4/v5 的 UDP-over-TCP 跑在 v4 帧流上（每个 datagram = 一个 v4 帧，明文同 v3：`UDPForward(0x01)|addr|payload`），握手头 `proto|CommandUDP(6)|0` 随首帧（带 salt + 初始 padding）发出；与 v3 不同，v4 在握手后**立即读一个命令响应字节**（`Tunnel`/`Error`，可能与首个回程 datagram 合帧，故 `recv` 用 pending 暂存余下数据），server salt 随首个回程帧到达后**惰性派生**读 cipher；`SnellUdpAssoc` 按 version 分派 v3(`SnellUdp`)/v4·v5(`SnellV4Udp`)，`supports_udp` 放开为 `version>=3`；独立 fake v4 UDP server 互通测试 `tests/snell_v4_udp.rs`（IPv4/IPv6/域名、大包、单关联多包、v5 当 v4、Routed）。**v4/v5 会话复用 / 连接池已完成**：新增 `reuse` 配置项，`reuse:true` 时 v4/v5 发 `CommandConnectV2` 并接入同一个按 server-key 的进程级连接池（池值改成枚举：v1-v3 shadowaead chunk 会话 / v4·v5 帧会话，version 在 key 里故不串），半关闭用 v4 的**零 payload 帧**（`payLen==0`，对端逻辑 EOF）代替 v3 的零长 chunk，`poll_shutdown` 发零帧但不关 TCP、`Drop` 在双向零帧干净交换且无残留时归还池；复用流重发请求头帧（不再带 salt/初始 padding）、重读一个命令响应字节，跨流 write/read cipher+nonce 连续推进；`reuse:false` 仍一次性新拨（行为同 PR4）。独立 fake v4 reuse server 互通测试 `tests/snell_v4_reuse.rs`（多个顺序 SOCKS5 流共用一条 TCP + 复用大包 + v5 配 reuse + `reuse:false` 每流新拨）。独立 fake v4 server 互通测试 `tests/snell_v4.rs`（小包 + 50KB 多帧大包 + v5 当 v4 + 校验首帧带 salt+初始 padding）+ 单元测试（v4/v5 选帧路径、v5 归一、v4/v5 拒绝 UDP） |
| wireguard | ✅ | ✅ | **L3 加密隧道 + 用户态网络栈数据面（多 peer，TCP + UDP relay）已完成**：与流代理不同，WireGuard 是到 peer 的一条 Noise_IKpsk2 隧道承载任意 IP 包；Noise 握手 / transport 封包 / rekey·cookie·keepalive 定时器交给 vetted `boringtun`（`noise::Tunn`，它不含网络/隧道栈），内核自己补：真实 UDP socket 收发 + `smoltcp` 用户态 TCP/IP 栈（沿用 TUN inbound 的 `Device`/poll 模型）+ per-config 设备注册表（同一 peer 的并发连接共享一条隧道 + 栈，仿 anytls 注册表）。每条被代理的 TCP = 栈内一个 smoltcp socket，其 IP 包经 `Tunn::encapsulate` 加密后 UDP 发给 peer，peer 回来的 UDP 经 `decapsulate` 解出 IP 包喂回栈；poll 循环 `tokio::select!` UDP 收 / 命令（开流）/ `update_timers` / 唤醒，流桥接用有界 channel + `Notify`（poll_write 满则停 waker，loop 排空后唤醒），peer FIN → 关 read 端 EOF，调用方 shutdown/drop → 关 write 端 FIN socket；配置字段对齐 Clash/mihomo（`private-key`/`public-key`/`pre-shared-key`/`ip`/`ipv6`/`mtu`/`reserved`/`persistent-keepalive`/`allowed-ips`/`peers`），接口按 peer 下发地址挂在 prefix 0（隧道是唯一 egress），`reserved` 3 字节按需打在消息头 1..4；独立 fake WireGuard server 互通测试 `tests/wireguard_outbound.rs`（第二个 `Tunn` 当 responder + 第二套 smoltcp 栈回显，真实 Noise 握手 + 小包/64KiB 多帧大包 round trip + 配置解析校验）。**UDP relay（PR8b-1）已完成**：每条 UDP 关联 = 同一 per-config 设备里的一个栈内 smoltcp UDP socket，逐目的一条关联（`send`/`recv`），datagram 经 `Tunn::encapsulate` 加密 UDP 发 peer、回程 `decapsulate` 解出喂回栈，1 datagram ↔ 1 内层 UDP 包，满队列丢包（UDP 有损语义），接入共享 `UdpEgress::WireGuard` / `run_wireguard_egress` 与 SS/SSR/Snell 同构；fake server 加 UDP 回显，互通测试覆盖握手暖机（UDP 无重传，按真实客户端重发探测包直到隧道建立）+ 多包/多百字节 datagram round trip。**隧道侧 DNS（`remote-dns-resolve`，PR8b-2）已完成**：开关开启 + 配 `dns` 解析器后，域名目标不再用宿主解析器，而是在隧道内向 `dns`（默认 53 端口，逐解析器尝试）发 DNS query 解析——复用上面的栈内 UDP 关联当传输（`A`/`AAAA` 按下发隧道地址族择序，UDP 无重传故每查询重发数次覆盖握手暖机），`hickory-proto` 只做 DNS wire 编解码，得到 IP 后再开 TCP/UDP relay；互通测试 fake server 加 UDP/53 应答 A=内层 IP，覆盖"域名经隧道解析→relay 回显"+ 开关无 `dns` 时报错校验。**rekey/keepalive 长稳（PR8b-3）已完成**：`Tunn::update_timers`（rekey/keepalive/握手重传）从 poll 循环顶按壁钟 cadence（`TIMER_TICK` 120ms）驱动，而非只放在 `select!` 超时臂——避免稳定 relay 流量下 `udp.recv`/`wake` 臂始终 ready 饿死定时器导致长连接错过 rekey 而断；`drive_timers` 有界排空同一 tick 多个到期动作；`persistent-keepalive` 已透传 `Tunn::new`；互通测试设 `persistent-keepalive: 1` + 服务端收包计数，验证空闲期客户端持续发 keepalive 且长空闲后连接仍 round-trip。**多 peer（PR8b-4）已完成**：顶层 peer + `peers` 列表各跑独立 Noise 会话 + UDP 端点，内层包按最长匹配 `allowed-ips` 前缀路由到对应 peer（顶层缺省 catch-all），设备注册表键含全 peer 指纹；多 peer 互通测试（两台带 tag 的 fake server 各占一个 /24，验证落到正确 peer）。**amnezia-wg 混淆（PR8b-5）已完成**：`amnezia-wg-option`（`jc`/`jmin`/`jmax`/`s1`/`s2`/`h1`-`h4`，全设备共用）——握手前发 `jc` 个 `jmin`-`jmax` 随机长垃圾包、握手 init/response 前缀 `s1`/`s2` 随机 padding、4 字节消息类型头按 `h1`-`h4` 改写（boringtun 仍产标准消息，混淆在其字节出栈时施加、解封前还原，Noise 引擎不变）；RX 按 `(padding+长度, 头)` 签名识别消息类型并剥除 padding、还原类型字节，垃圾/无法识别包丢弃；`h1`-`h4` 校验须互异且 >4；互通测试 fake server 施加同套混淆验证 TCP round trip |
| ssr | ✅ | ✅ | 完整 SSR 三层栈：**流密码**（`aes-128-cfb`/`aes-256-cfb`/`chacha20-ietf`/`rc4-md5`/`none`，EVP_BytesToKey KDF + 随机 IV）+ **协议**（`origin`/`auth_aes128_sha1`/`auth_aes128_md5`/`auth_chain_a`，HMAC+AES-ECB 认证头 + 每包校验 + xorshift128plus 随机 padding）+ **混淆**（`plain`/`http_simple`/`tls1.2_ticket_auth`，伪 HTTP GET / 伪 TLS 1.2 Client Hello）；TCP relay + **UDP relay**（逐包独立加密：随机 IV + 一次性流密码，协议层 UDP 帧 `origin`/`auth_aes128_*`(uid+HMAC[:4])/`auth_chain_a`(RC4 + xorshift padding + 1 字节 HMAC)，obfs 在 UDP 下不参与）；独立 fake SSR server 互通测试 `tests/ssr_outbound.rs`（TCP，5 cipher × 4 protocol × 3 obfs）+ `tests/ssr_udp.rs`（UDP，5 cipher × 4 protocol）。注：这些老式流密码（CFB/RC4/ChaCha20 无 AEAD）是 §3 中标 ❌ 的，SSR 数据面专用，不暴露给 SS 出站 |

**解析但无数据面（导入显示 OK，跑不通）：** `hysteria / ssh / masque / gost-relay / trusttunnel / openvpn / tailscale / mieru / sudoku / dns`。（wireguard 已补 TCP + UDP 数据面，见上表 wireguard 行；http 上游代理已补 TCP 数据面，见上表 http 行。）

## 2. 传输 × 安全（VMess / VLESS / Trojan 共用 `transport::build_layers`）

| 传输 (network) | 状态 | 备注 |
|---|---|---|
| tcp | ✅ | 默认 |
| ws | ✅ | ws-opts: path / headers / Host |
| httpupgrade | ✅ | ws + `v2ray-http-upgrade: true` |
| grpc | ✅ | 自动加 h2 ALPN |
| xhttp | ✅ | `stream-one`（全双工单 POST）+ `stream-up`（流式 POST 上 / 独立 GET 下）+ `packet-up`（逐包 POST + seq）；session-id 路径关联、`x_padding` 填充；`auto`/空 → stream-one |
| h2 | △ | **仅 over TLS/REALITY**（无 TLS 则 ❌） |
| 其它 network | ❌ | bail "not implemented" |

| 安全 (security) | 状态 | 备注 |
|---|---|---|
| none | ✅ | |
| tls | ✅ | sni/servername、alpn、skip-cert-verify、client-fingerprint（uTLS 指纹整形） |
| reality | ✅ | 需 servername + reality-opts.public-key(32B)，short-id ≤8B |
| ech (ech-opts) | ✅ | `enable` + base64 `config`(ECHConfigList) → rustls `with_ech`；内置 RFC 9180 HPKE provider（X25519+HKDF-SHA256 + 3 种 AEAD，过官方测试向量）。`query-server-name`：缺 `config` 时在握手前向 `1.1.1.1:53` 查该名的 `HTTPS`(type 65) 记录、取 `ech` SvcParam 当 ECHConfigList（`hickory-proto` 解析）；两者皆缺才报错 |

> flow：仅 VLESS 的 `xtls-rprx-vision` ✅（且仅 raw TCP）；其它 flow / 其它协议带 flow → ❌。

## 3. Shadowsocks cipher

| cipher | TCP | UDP |
|---|---|---|
| aes-128-gcm | ✅ | ✅ |
| aes-256-gcm | ✅ | ✅ |
| chacha20-ietf-poly1305（别名 chacha20-poly1305） | ✅ | ✅ |
| **2022-blake3-aes-128-gcm / -aes-256-gcm / -chacha20-poly1305** | ✅ (PR #446) | ✅ (SIP022 UDP) |
| 老式流密码 aes-*-cfb / rc4-md5 等 | ❌（SS）/ ✅（SSR） | ❌ | SSR 数据面专用（见 §1 ssr 行）；SS 出站仍拒绝 |
| SIP003 plugin（simple-obfs http/tls、v2ray-plugin ws±tls / http-upgrade / mux / quic） | ✅ | — | v2ray-plugin 全模式已接：`websocket`（±tls）、`v2ray-http-upgrade`（复用 httpupgrade transport）、`mux`（mux.cool 单子连接帧封装）、`mode: quic`（标准 QUIC + 强制 TLS，ALPN `[h2,http/1.1]`，按 server 进程级连接池复用、每 relay 一条 bi-stream）；其余 mode（grpc 等）仍拒绝 |

## 4. 入站

- **mixed inbound**：HTTP CONNECT (`inbound/http.rs`) + SOCKS5 (`inbound/socks5.rs`)，含 SOCKS5 UDP ASSOCIATE。
- **TUN** (`tun/`)：TCP + UDP。

---

## 5. 缺口 & 建议优先级

> 评估轴：**价值**（命中多少真实节点 / 对"链路顺畅度"的影响）× **工作量** × **互通风险**。

| # | 缺口 | 价值 | 工作量 | 风险 | 建议 |
|---|---|---|---|---|---|
| 1 | ~~SS `2022-blake3-*` TCP~~ | 高（现代 SS 主流） | 中 | 低 | ✅ **已完成 (PR #446)** |
| 2 | ~~SS `2022-blake3-*` UDP~~ | 中（UDP-over-SS：节点内跑 QUIC/HTTP3、游戏、WebRTC、DNS；full-cone NAT） | 中 | 中（separate-header：gcm 用 AES-ECB 头加密、chacha 用 XChaCha20，跟 TCP 完全不同的封装） | ✅ **已完成**（SIP022 UDP：AES separate-header + XChaCha20-Poly1305） |
| 3 | ~~**SS SIP003 plugin**（obfs / v2ray-plugin 的 ws/tls 混淆）~~ | 中（带混淆的 SS 节点直接断） | 中 | 中 | ✅ **已完成**（simple-obfs http + 伪 TLS、v2ray-plugin websocket + 可选 TLS；复用现有 ws/tls transport）。✅ **v2ray-plugin 非 websocket / mux 已补**：`v2ray-http-upgrade`（复用 httpupgrade transport）、`mux: true`（mux.cool 单子连接 New/Keep-Data/End 帧封装，对齐 mihomo `mux.go` 线格式）、`mode: quic`（复用 quinn QUIC transport + 强制 TLS + 按 server 连接池复用 + 每 relay bi-stream，ALPN 默认 `[h2,http/1.1]`）；互通测试见 `tests/shadowsocks_plugin.rs`（mux / http-upgrade / quic / quic 连接池）|
| 4 | ~~**ECH 接线**（`ech-opts` → 实际握手）~~ | 低-中（少量启用 ECH 的节点） | 中 | 中 | ✅ **已完成**（自实现 RFC 9180 HPKE provider 桥接 rustls `with_ech`；ring 后端无 HPKE，故用 x25519-dalek+hkdf+aes-gcm/chacha20poly1305 手搓 base 模式并过 RFC 测试向量）。✅ **`query-server-name` DNS 拉取已完成**（缺 `config` 时握手前查 `HTTPS` 记录的 `ech` SvcParam，连接时异步 UDP 查询默认 `1.1.1.1:53`；fake resolver 单测覆盖解析/拉取/无记录报错/端到端 SNI 隐藏） |
| 5 | VMess 老式 alterId(MD5) | 低（旧 VMess，已淘汰） | 低 | 低 | **不建议补** |
| 6 | ~~xhttp packet-up/stream-up 模式~~ | 低（小众） | 低-中 | 中 | ✅ **已完成**（`stream-up` + `packet-up`：单 h2 连接上 GET 下行 + POST 上行，对齐 Xray splithttp 线格式；独立 fake h2 server 互通测试 `tests/xhttp_multi.rs`） |
| 7 | **TUIC / Hysteria2（QUIC 数据面）** | **高（2026 抗封锁前沿）** | **大**（引入 QUIC 栈 quinn + 协议层） | 高 | ✅ **TUIC v5 已完成**（TCP relay `tests/tuic_outbound.rs` + UDP relay `tests/tuic_udp_outbound.rs`）。✅ **Hysteria2 已完成**（TCP relay `tests/hysteria2_outbound.rs` + UDP relay `tests/hysteria2_udp_outbound.rs`）。**UDP 走 QUIC datagram**（Hysteria2 datagram + TUIC `Packet`，含 >MTU 分片/重组，共享 `protocols/quic_udp.rs`；独立 fake QUIC server 互通测试覆盖单包 + 分片）。✅ **Hysteria2 Salamander obfs + 端口跳跃已完成**（`obfs: salamander` 用 `BLAKE2b-256(psk‖salt)` keystream 逐 datagram XOR 混淆，`ports`/`hop-interval` 在 QUIC 之下的自定义 `AsyncUdpSocket`（`transport/quic_obfs.rs`）里跳端口；独立 fake Salamander server 互通测试 `tests/hysteria2_obfs.rs`）。✅ **TUIC / Hysteria2 0-RTT 已完成**（`reduce-rtt: true`：进程级 rustls session ticket 缓存 + `quinn::Connecting::into_0rtt`；TUIC 把无密的 `Connect` 头作为早期数据、RFC 5705 鉴权 token 等握手完成后再发，Hysteria2 把幂等的 HTTP/3 `/auth` + `TCPRequest` 作为早期数据；服务端拒绝 0-RTT 时自动回退 1-RTT 重发；互通测试 `tests/tuic_zero_rtt.rs` + `tests/hysteria2_zero_rtt.rs` 覆盖首拨 1-RTT + 续拨 0-RTT） |
| 8 | ~~WireGuard~~ / ~~ssr~~ / snell / anytls 数据面 | 视订阅而定 | 大（各自独立工程） | 中-高 | ✅ **anytls 已完成**。✅ **snell 已完成**（TCP；UDP `CommandUDP` v3 已补，见 §1 snell 行 + `tests/snell_udp.rs`）。✅ **ssr 已完成**（完整三层栈：流密码 + 协议 + 混淆；5 cipher × 4 protocol × 3 obfs；TCP + UDP relay（UDP 逐包独立加密 + 协议层 UDP 帧，obfs 不参与）；独立 fake SSR server 互通测试 `tests/ssr_outbound.rs` + `tests/ssr_udp.rs`；老式流密码 SSR 专用、不暴露给 SS 出站）。✅ **WireGuard 已完成（单 peer，TCP relay PR8a + UDP relay PR8b-1 + 隧道侧 DNS PR8b-2）**：`boringtun` Noise 引擎 + `smoltcp` 用户态栈，TCP/UDP 包经隧道加密 UDP 发 peer，`remote-dns-resolve` 时域名在隧道内向 `dns` 解析器解析（栈内 UDP + `hickory-proto` 编解码），独立两-`Tunn` + smoltcp 回显互通测试 `tests/wireguard_outbound.rs`（rekey/keepalive 长稳 PR8b-3 已完成：定时器从 poll 循环顶按壁钟 cadence 驱动不被流量饥死）；**多 peer（PR8b-4）已完成**：顶层 peer + `peers` 列表各跑自己的 Noise 会话 + UDP 端点，内层包按最长匹配 `allowed-ips` 前缀路由到对应 peer（顶层 peer `allowed-ips` 缺省 catch-all `0.0.0.0/0`+`::/0`），设备注册表键含全 peer 指纹，多 peer 互通测试（两台带 tag 的 fake server 各占一个 /24，验证内层包落到正确 peer）；**amnezia-wg 混淆（PR8b-5）已完成**（`amnezia-wg-option`：握手前 `jc` 个随机垃圾包 + init/response 的 `s1`/`s2` 前缀 padding + `h1`-`h4` 消息头改写，TX 施加 / RX 还原，boringtun Noise 引擎不变；互通测试 fake server 施加同套混淆）；anytls UDP 已补；anytls padding scheme 流量整形已补（默认 `padding-md5` DSL + `cmdWaste` 分包填充），`cmdUpdatePaddingScheme` 动态应用已补（按 `server:port` 进程级存储，后续新连接按下发 scheme 整形），会话池复用 + 并发多 stream 多路复用已补（后台 reader/writer 双任务 demux + per-server 会话注册表，一条 TLS 连接并发承载多流） |

---

## 6. 结论 & 待定决策

**已接通的主线**：SS(AEAD，含 2022 TCP+UDP) / VMess / VLESS / Trojan × `tcp/ws/grpc/xhttp(stream-one/stream-up/packet-up)/h2(over TLS)/httpupgrade` × `none/tls/reality`（+ VLESS Vision，raw TCP；+ TUIC v5 与 Hysteria2 over QUIC，TCP relay **及 UDP relay（QUIC datagram）**）。这套已经覆盖绝大多数现代订阅的 TCP + UDP 链路。

**接下来的岔路口（待 owner 拍板）：**
0. ~~#8 AnyTLS 数据面~~（已完成，TLS 会话层多路复用 + SocksAddr 代理；TCP+UDP relay）、~~#8 Snell 数据面~~（已完成，Shadowsocks-AEAD 分块帧 + Argon2id 子密钥；v1/v2/v3；TCP relay）。~~#8 WireGuard 数据面~~（已完成，单 peer TCP relay PR8a + UDP relay PR8b-1 + 隧道侧 DNS PR8b-2：boringtun Noise + smoltcp 用户态栈，栈内 UDP socket → 隧道，`remote-dns-resolve` 域名走隧道解析，rekey/keepalive 长稳 PR8b-3（定时器壁钟 cadence 驱动不被流量饥死），多 peer PR8b-4（`peers` + 按 `allowed-ips` 最长前缀路由，每 peer 独立 Noise 会话 + UDP 端点），amnezia-wg 混淆 PR8b-5（`amnezia-wg-option`：垃圾包 + `s1`/`s2` padding + `h1`-`h4` 头改写，TX 施加 / RX 还原））。~~#8 SSR 数据面~~（已完成，完整三层栈：EVP_BytesToKey+随机IV 流密码 / auth_aes128·auth_chain_a 协议 / http_simple·tls1.2_ticket_auth 混淆；TCP + UDP relay）。
1. **继续补"已有 SS"** → ~~#2 SS-2022 UDP~~（已完成）、~~#3 SIP003 plugin~~（v2ray-plugin ws/tls + simple-obfs http/tls 全部完成）。稳、低风险。

> 已补 `http`(s) 上游代理出站（HTTP `CONNECT` + 可选 Basic 认证 + 可选 TLS，与 `socks5` 上游同构、低风险），「解析但无数据面」清单里的 `http` 已移除。
2. **直接上 QUIC 系新协议** → ~~#7 TUIC~~（TUIC v5 TCP relay 已完成，引入 quinn QUIC 栈）、~~Hysteria2~~（HTTP/3 鉴权 + 裸 QUIC 流 TCP relay 已完成）、~~两者 UDP relay~~（QUIC datagram：Hysteria2 datagram + TUIC `Packet`，含分片/重组，已完成）；~~Hysteria2 的 Salamander obfs/端口跳跃~~（已完成）；~~TUIC/Hysteria2 的 0-RTT~~（`reduce-rtt`：session ticket 缓存 + `into_0rtt` 早期数据，已完成）。价值最高但工作量最大。
3. **补已有传输的洞** → ~~#4 ECH 接线~~（已完成，含 `query-server-name` DNS 拉取 ECHConfig）、~~#6 xhttp stream-up/packet-up~~（已完成）。

> 建议在拍板前先确认**真实订阅里的协议分布**：如果大量是 hysteria2/tuic/reality，则把精力投向 #7 比补 SS-2022 UDP 更划算；如果仍以 SS / VMess / Trojan 为主，则按 #2 → #3 顺序补齐"已有"更稳。
