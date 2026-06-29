# learn-gripe 内核可维护性审计

> **状态：第一轮（目录分层）已全部完成（2026-06）。** 下文 §1–§4「现状画像」记录的是重构*之前*的平铺结构，保留作为本轮重构的动机与依据记录。落地 PR：#431/#432（目录化）、#433（routing 拆分）、#434（conntrack + proxy）、#435（UDP egress 收敛）、#436（tun.rs 拆分）。当前 `crates/learn-gripe/src` 已按 `protocols/ transport/ inbound/ routing/ dns/ tun/ conntrack/ config/` 分层。
>
> **第二轮（待办）见 §5：每协议「身份」散落在十余处平行 `match`/列表，新增协议需手工同步全部站点，已造成过真实漂移 bug。** 这是协议数量增长后的首要可维护性风险。
>
> 目的：理清 `crates/learn-gripe` 各文件职责、识别混合关注点与拆分点，给出建议的模块分层。
> 范围：**只做分析与建议，不改任何代码**。理清后再对应前后端。

## 1. 现状画像

- 约 **12,350 行 / 27 个模块**，全部平铺在 `src/*.rs`，**零子目录**。`lib.rs` 用 27 条 `mod` + 一批 `pub use` 把它们拍平。
- 平铺结构本身是首要可维护性问题：从文件树看不出协议 / 传输 / 路由 / DNS / TUN / 进出站之间的分层关系，全靠在 `lib.rs` 的 `pub use` 注释里人肉分组。

### 生产代码行数（已剔除测试，关键指标）

| 模块 | 总行 | 代码 | 测试 | 职责 |
|---|---:|---:|---:|---|
| tun | 929 | **867** | 62 | TUN 数据面（smoltcp 设备 + TCP 流 + UDP 流 + DNS 拦截 + relay） |
| router | 2255 | **794** | 1461 | 路由：traits + 值类型 + 匹配器 + Router |
| vmess | 985 | 750 | 235 | VMess 出站协议 |
| vision | 727 | 630 | 97 | XTLS Vision 流过滤器 |
| shadowsocks | 720 | 595 | 125 | SS 出站协议 |
| dns | 706 | 537 | 169 | DNS server + Fake-IP + 统计 |
| tls | 622 | 450 | 172 | TLS + REALITY |
| conntrack | 576 | 429 | 147 | 连接表 + Counted 流 + relay_tracked |
| proxy | 363 | 363 | 0 | 出站配置 DTO（ProxyOptions + 各 *Opts） |
| udp | 355 | 355 | 0 | UDP associate 中继 + proxy framing |
| transport | 369 | 341 | 28 | 流栈装配（按 security/transport 组装） |
| grpc | 346 | 314 | 32 | gRPC 传输 |
| socks5 | 335 | 279 | 56 | SOCKS5 编解码（入站 + 出站客户端） |
| vless | 458 | 244 | 214 | VLESS 出站协议 |
| server | 233 | 233 | 0 | GripeKernel/Handle，本地监听编排 |
| http | 347 | 233 | 114 | HTTP CONNECT 入站 |
| trojan | 352 | 191 | 161 | Trojan 出站协议 |
| delay | 248 | 176 | 72 | 时延测速（URL test） |
| ws | 183 | 172 | 11 | WebSocket 传输 |
| httpupgrade | 201 | 172 | 29 | HTTPUpgrade 传输 |
| config | 261 | 155 | 106 | GripeConfig / OutboundMode |
| outbound | 178 | 134 | 44 | 出站 connect 分发 + UDP egress 选择 |
| obfuscation | 174 | 109 | 65 | 简单混淆 |
| h2stream | 151 | 151 | 0 | HTTP/2 流实现 |
| xhttp | 88 | 77 | 11 | XHTTP 传输 |
| http2 | 71 | 60 | 11 | H2 传输配置 |
| address | 45 | 45 | 0 | TargetAddr 基础类型 |

**纠正一个常见误判**：`router.rs` 看着 2255 行像“上帝文件”，但其中 **1461 行是测试**，生产代码只有 794 行。真正按生产代码量最大的是 `tun.rs`（867 行）。

## 2. 混合关注点 / 拆分点（按优先级）

### P0 — `tun.rs`（867 代码行）：单文件塞了 4~5 个正交职责
一个文件同时包含：
- smoltcp `Device` 实现（`TunPhy` / `PhyRxToken` / `PhyTxToken`）
- TCP 流生命周期（`Flow` / `new_flow_for_syn` / `run_flow` / `parse_tcp_endpoints`）
- UDP 流生命周期 + 3 个中继变体（`run_udp_session` / `run_udp_direct` / `run_udp_ss` / `run_udp_proxy`）
- TUN 内 DNS 拦截应答（`answer_dns`）
- 帧编解码（`parse_udp_datagram` / `build_udp_reply_frame`）

建议拆为 `tun/{mod, device, tcp, udp, dns_intercept}`。

### P0 — UDP 逻辑分散在 3 个文件，存在并行实现
- `tun.rs`：`run_udp_direct / run_udp_ss / run_udp_proxy`
- `udp.rs`：`run_direct_egress / run_ss_egress / run_proxy_egress` + `ProxyFraming`
- `outbound.rs`：`resolve_udp_egress / connect_proxy_udp / supports_udp_associate`

“把 UDP 中继到 direct/ss/proxy”这件事在 TUN 侧和 SOCKS-associate 侧各写了一套。这是**最值得收敛的重复点**——应抽出统一的 UDP egress 模块（`outbound/udp.rs`），TUN 与 associate 共用。**需逐行确认两套是否语义等价后再合并**。

### P1 — `router.rs`：4 个关注点混在一个文件
- 集成边界 traits：`GeoLookup` / `RuleSetLookup` / `ProcessLookup`（+ `ProcessInfo`）
- 纯值类型：`IpCidr` / `PortRange` / `UidRange` / `masked()`
- 匹配引擎：`RuleMatcher` 枚举 + 匹配逻辑 + `domain_has_suffix`
- 编排：`Rule` / `Router` / `Selection`

建议拆为 `routing/{mod(Router), matcher, types, lookup}`，1461 行测试随对应模块迁移。

### P1 — `conntrack.rs`：连接表 + 通用 IO 适配器混放
- 连接计量：`ConnRegistry` / `ConnMeta` / `ConnSnapshot` / `TrackedConn`
- 通用 IO：`Counted<S>`（流字节计数 adapter）+ `relay_tracked`（双向中继）

后者是与“连接表”无关的 IO 中继原语。建议分到 `net/relay.rs`，连接表留 `net/conntrack.rs`。

### P2 — `proxy.rs`：363 行全是出站配置 DTO
`ProxyEntry` / `ProxyType` / `ProxyOptions` + `WsOpts/H2Opts/GrpcOpts/RealityOpts/EchOpts/...`。本身职责单一（配置面），但应改名/归位为出站配置面（`config/outbound_opts.rs` 或 `protocols/options.rs`），与运行时协议实现分开。

### P2 — HTTP 家族命名歧义
`http.rs`（入站 HTTP 代理）vs `http2.rs`/`h2stream.rs`（H2 传输）vs `httpupgrade.rs` vs `xhttp.rs`（传输）。文件名相近但分属“入站监听”和“传输层”。归入 `inbound/` 与 `transport/` 子目录后歧义自然消除；`http2.rs`+`h2stream.rs` 可合为 `transport/h2.rs`。

### 备注（非拆分，仅说明）
- `socks5.rs` 同时含入站握手（`server_handshake/read_request`）与出站客户端编解码（`client_connect/encode_address`）——它实际是被入站监听和出站链式代理共用的 SOCKS codec，放哪都需被双方引用，建议作为共享 codec（可留 `inbound/` 或独立 `socks5` 模块）。

## 3. 建议的目标分层

```
src/
  lib.rs
  config.rs              GripeConfig / OutboundMode
  address.rs             TargetAddr（共享基础类型）

  routing/
    mod.rs               Router / Rule / Selection
    matcher.rs           RuleMatcher + 匹配逻辑
    types.rs             IpCidr / PortRange / UidRange / masked
    lookup.rs            GeoLookup / RuleSetLookup / ProcessLookup (+ ProcessInfo)
    delay.rs             时延测速

  dns/
    mod.rs               DnsServer / DnsHandle / DnsMode / DnsConfig
    fakeip.rs            FakeIpPool / FakeIpConfig / unmap_fake_ip
    stats.rs             DnsStats + snapshots

  inbound/
    mod.rs               server.rs：GripeKernel / GripeHandle / serve
    socks5.rs            （共享 SOCKS codec）
    http.rs              HTTP CONNECT 入站

  outbound/
    mod.rs               connect 分发 + UDP egress 选择（原 outbound.rs）
    udp.rs               统一 UDP egress + framing（合并 udp.rs 与 tun 内 UDP 中继）
    options.rs           原 proxy.rs 的出站配置 DTO

  protocols/
    vmess.rs vless.rs shadowsocks.rs trojan.rs vision.rs

  transport/
    mod.rs               流栈装配（原 transport.rs）
    tls.rs ws.rs grpc.rs h2.rs httpupgrade.rs xhttp.rs obfuscation.rs

  tun/
    mod.rs               serve_tun / serve_tun_device / run 循环 / 接口构建
    device.rs            TunPhy + Rx/TxToken
    tcp.rs               Flow / run_flow / TCP 端点解析
    udp.rs               UdpFlow / run_udp_session（复用 outbound/udp）
    dns_intercept.rs     answer_dns

  net/
    conntrack.rs         ConnRegistry / ConnMeta / snapshots / TrackedConn
    relay.rs             Counted<S> / relay_tracked
```

## 4. 执行建议（如果决定动手，分批做、每批可独立编译+过 CI）

1. **纯搬迁（零行为变更）**：建立子目录、`mod` 重组，`lib.rs` 的 `pub use` 保持对外 API 不变。先做 protocols/、transport/、inbound/、dns/、routing/ 的目录化，每个目录一个 PR，diff 基本是 `git mv` + 改 `mod` 路径，风险最低。
2. **router.rs 内部拆分**：把 traits/types/matcher 拆出，测试随迁。
3. **tun.rs 内部拆分**：device/tcp/udp/dns_intercept。
4. **UDP egress 收敛（唯一有真实行为风险的一步）**：逐行核对 `tun.rs` 与 `udp.rs` 的 direct/ss/proxy 三套中继是否等价，再合并；单独 PR，重点测试。
5. **conntrack 拆 relay**、**proxy.rs 改名归位**：低风险收尾。

> 对外 API（`GripeHandle/GripeKernel/Router/DnsHandle/serve_tun` 等）全部由 `lib.rs` 的 `pub use` 重新导出，因此以上重组**不影响 src-tauri / 前后端调用面**——前后端对接可在结构理清后再做，互不阻塞。

---

## 5. 第二轮：每协议分发逻辑散落（2026-06 之后新增）

第一轮把*文件*按职责分了层，但**单个出站协议的「身份」仍横向散落在十余处平行的 `match`/列表里**，彼此靠人肉保持同步。协议数量从个位数涨到 20+ 后，这已成为新增协议时最易出错、也最容易悄悄漂移的点。

### 5.1 一个协议要改的站点（当前 ≈ 12 处）

新增一个出站协议（以最近的 masque / sudoku 为例），必须在以下每一处各加一行/一臂，缺一处就行为不一致：

| 文件 | 站点 | 作用 |
|---|---|---|
| `config/mod.rs` | `OutboundMode` 枚举 | 出站类型变体 |
| `config/mod.rs` | `OutboundMode::from_proxy` | 配置 → 出站构造 |
| `config/mod.rs` | `direct_dial_endpoints` | TUN 全局捕获的 bypass 端点 |
| `config/mod.rs` | `type_label` | 连接簿记标签 |
| `config/mod.rs` | `supports_global_capture` | 能否做 TUN 默认路由 |
| `outbound.rs` | `connect` | TCP 出站分发 |
| `outbound.rs` | `UdpEgress` 枚举 | UDP egress 变体 |
| `outbound.rs` | `supports_udp_associate` | 能否 UDP ASSOCIATE |
| `outbound.rs` | `resolve_udp_egress` | UDP egress 选择 |
| `outbound.rs` | `connect_proxy_udp` | UDP 隧道流建立 |
| `config/outbound_opts.rs` | `ProxyType` 枚举 + `ProxyEntry::support()` | 前端「是否已实现」信号 |
| `src-tauri/.../lifecycle.rs` | `outbound_label` | app 侧标签 |

### 5.2 已发生的真实漂移（已修，留作证据）

`ProxyEntry::support()` 曾用 `_ => Unsupported` 兜底，长期与 `from_proxy` 脱节：**tuic / hysteria2 / anytls / snell / masque / sudoku / wireguard 的数据面早已接好，却被报成 `Unsupported`**，前端拿到的「是否可用」信号是错的。

- 修复 PR：**#490** —— 把 `support()` 改成对 `ProxyType` 的穷尽 `match`（去掉通配），并加 `tests/proxy_schema.rs::support_matches_from_proxy`：遍历每个 `ProxyType`，断言 `support()==Implemented` 当且仅当 `from_proxy` 认识该类型。这是把「同一份事实的两处副本」用测试钉死的**止血手段**，但根因（多处副本）仍在。

### 5.3 待办：用 trait + 注册表收敛分发

目标：让「一个协议」= **一处定义**，而不是十二处平行分支。

1. **引入 `Outbound` trait**，每协议实现一次：
   ```rust
   trait Outbound {
       fn label(&self) -> &'static str;
       fn endpoint(&self) -> (String, u16);          // direct_dial_endpoints
       fn supports_global_capture(&self) -> bool;
       async fn connect_tcp(&self, target: &TargetAddr) -> Result<BoxedStream>;
       fn udp_capability(&self) -> UdpCapability;     // 取代 supports_udp_associate / resolve_udp_egress / connect_proxy_udp 的分散判断
   }
   ```
   `connect` / `type_label` / `direct_dial_endpoints` / `supports_global_capture` / `supports_udp_associate` / `resolve_udp_egress` / `connect_proxy_udp` 由对 trait 对象的统一调用取代，~10 处 `match` 压成 1~2 处。
2. **`from_proxy` 收敛为注册表**：`ProxyType → fn(&ProxyEntry) -> Result<Box<dyn Outbound>>` 的单一映射表；`support()` 直接由「该类型在表中是否存在」派生，§5.2 的漂移从结构上不再可能。
3. **穷尽性兜底**：保留 §5.2 的 `support_matches_from_proxy` 思路；对 trait 化后无法用类型系统覆盖的列表（如 app 侧 `outbound_label`），补 over-`ProxyType` 的穷尽测试。

> 行为风险：UDP 侧（`UdpEgress` / QUIC datagram vs proxy-stream vs 裸 UDP socket 三类语义）合并需逐协议核对，建议单独 PR、重点测试，与 TCP 收敛分开做。

### 5.4 顺带：超出仓库 800 行上限的协议巨石文件

`ssr.rs`(2095) · `snell.rs`(2178) · `anytls.rs`(1682) · `wireguard.rs`(1671)。功能无误，但应按 `protocols/sudoku/` 的范式（obfs/record/kip/table/… 拆成 8 个小文件）拆到各自的 `protocols/<name>/` 子目录。低风险、可独立 PR。

### 5.5 建议执行顺序（每步独立 PR、独立过 CI）

1. ✅ **止血**：修 `support()` 漂移 + 穷尽测试（PR #490，已合并）。
2. ✅ **TCP 分发 trait 化**（PR #492，已合并）：引入 `TcpOutbound` trait（`type_label`/`dial_endpoint`/`supports_global_capture`/`connect_tcp`），各协议用 `impl_tcp_outbound!` 宏实现一次；`OutboundMode::as_tcp_outbound` 成为唯一穷尽映射，`connect`/`type_label`/`direct_dial_endpoints`/`supports_global_capture` 全部经它分发。`from_proxy` 仍是注册映射（暂未做 `Box<dyn>` 注册表）。
3. ✅ **UDP 分发收敛**（PR #493）：`supports_udp_associate` 与 `resolve_udp_egress` 的协议清单收敛到单一 `udp_egress_for` 真源（前者由后者派生，加 `supports_udp_associate_tracks_resolve_egress` 守护测试）；`run_egress` 去掉通配、改为穷尽匹配。**注意**：UDP egress 运行器（`run_egress`/`run_*_egress`）对 `ReplySink` 泛型，不是对象安全的，无法并入 `dyn` trait，因此 `UdpEgress` 仍保留为枚举（QUIC datagram / proxy-stream / 裸 UDP socket / udp-over-tcp 四类语义 + 各自 runner），由穷尽 match 防漂移——这是设计约束而非堆叠。
4. **拆巨石文件**：ssr / snell / anytls / wireguard 目录化。
