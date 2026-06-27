# learn-gripe 内核协议能力矩阵 & 优先级路线图

> 目的：在一处记录内核数据面「现在能跑什么、缺什么、接下来按什么顺序补」，作为后续排期的决策依据。
> 维度区分两件事：**能解析（parse）** vs **能跑流量（data plane）**。本文只关心后者——「能不能真正转发」。
> 来源：逐行核对 `crates/learn-gripe/src/{outbound,transport,protocols,udp,inbound}`。
> 图例：✅ 已实现并接通　❌ 显式 bail 拒绝（不会静默乱编码）　△ 部分　— 不适用
>
> 最近更新：SS `2022-blake3-*` 的 **TCP** 已合并（PR #446）；**UDP** 数据面已实现（SIP022：AES 系 separate-header + chacha 系 XChaCha20-Poly1305）。

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
| shadowsocks (ss) | ✅ | ✅ | cipher 限制见 §3；2017 系与 2022 系均 TCP+UDP；无 SIP003 plugin |
| trojan | ✅ | ✅ | 经 `build_layers` 全传输/安全 |
| vmess | ✅ | ✅ | 仅 alterId 0 (AEAD)；cipher auto / aes-128-gcm / chacha20-poly1305 |
| vless | ✅ | ✅ | 支持 Vision（仅 raw TCP）；encryption 须 none |

**解析但无数据面（导入显示 OK，跑不通）：** `ssr / snell / http / anytls / hysteria / hysteria2 / tuic / wireguard / ssh / masque / gost-relay / trusttunnel / openvpn / tailscale / mieru / sudoku / dns`。

## 2. 传输 × 安全（VMess / VLESS / Trojan 共用 `transport::build_layers`）

| 传输 (network) | 状态 | 备注 |
|---|---|---|
| tcp | ✅ | 默认 |
| ws | ✅ | ws-opts: path / headers / Host |
| httpupgrade | ✅ | ws + `v2ray-http-upgrade: true` |
| grpc | ✅ | 自动加 h2 ALPN |
| xhttp | △ | **仅 stream-one**；packet-up/down 等模式 ❌ |
| h2 | △ | **仅 over TLS/REALITY**（无 TLS 则 ❌） |
| 其它 network | ❌ | bail "not implemented" |

| 安全 (security) | 状态 | 备注 |
|---|---|---|
| none | ✅ | |
| tls | ✅ | sni/servername、alpn、skip-cert-verify、client-fingerprint（uTLS 指纹整形） |
| reality | ✅ | 需 servername + reality-opts.public-key(32B)，short-id ≤8B |
| ech (ech-opts) | ❌(实质) | 字段能解析，但 `build_layers` 未接线 → **未生效** |

> flow：仅 VLESS 的 `xtls-rprx-vision` ✅（且仅 raw TCP）；其它 flow / 其它协议带 flow → ❌。

## 3. Shadowsocks cipher

| cipher | TCP | UDP |
|---|---|---|
| aes-128-gcm | ✅ | ✅ |
| aes-256-gcm | ✅ | ✅ |
| chacha20-ietf-poly1305（别名 chacha20-poly1305） | ✅ | ✅ |
| **2022-blake3-aes-128-gcm / -aes-256-gcm / -chacha20-poly1305** | ✅ (PR #446) | ✅ (SIP022 UDP) |
| 老式流密码 aes-*-cfb / rc4-md5 等 | ❌ | ❌ |
| SIP003 plugin（obfs-local / simple-obfs / v2ray-plugin） | ❌ | ❌ |

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
| 3 | **SS SIP003 plugin**（obfs / v2ray-plugin 的 ws/tls 混淆） | 中（带混淆的 SS 节点直接断） | 中 | 中 | 复用现有 transport 层思路 |
| 4 | **ECH 接线**（`ech-opts` → 实际握手） | 低-中（少量启用 ECH 的节点） | 中 | 中（依赖 vendored rustls 的 ECH 支持程度） | 视订阅是否真的用到 |
| 5 | VMess 老式 alterId(MD5) | 低（旧 VMess，已淘汰） | 低 | 低 | **不建议补** |
| 6 | xhttp packet-up/down 模式 | 低（小众） | 低-中 | 中 | 视需求 |
| 7 | **Hysteria2 / TUIC（QUIC 数据面）** | **高（2026 抗封锁前沿）** | **大**（要引入 QUIC 栈 + 各自协议层） | 高 | 真正的"新协议"增量；建议作为独立大里程碑 |
| 8 | WireGuard / ssr / snell / anytls 数据面 | 视订阅而定 | 大（各自独立工程） | 中-高 | 按实际订阅命中再排 |

---

## 6. 结论 & 待定决策

**已接通的主线**：SS(AEAD，含 2022 TCP+UDP) / VMess / VLESS / Trojan × `tcp/ws/grpc/xhttp(stream-one)/h2(over TLS)/httpupgrade` × `none/tls/reality`（+ VLESS Vision，raw TCP）。这套已经覆盖绝大多数现代订阅的 TCP 链路。

**接下来的岔路口（待 owner 拍板）：**
1. **继续补"已有 SS"** → ~~#2 SS-2022 UDP~~（已完成）、#3 SIP003 plugin。稳、低风险。
2. **直接上 QUIC 系新协议** → #7 Hysteria2 / TUIC。价值最高但工作量最大。
3. **补已有传输的洞** → #4 ECH 接线 / #6 xhttp 其它模式。

> 建议在拍板前先确认**真实订阅里的协议分布**：如果大量是 hysteria2/tuic/reality，则把精力投向 #7 比补 SS-2022 UDP 更划算；如果仍以 SS / VMess / Trojan 为主，则按 #2 → #3 顺序补齐"已有"更稳。
