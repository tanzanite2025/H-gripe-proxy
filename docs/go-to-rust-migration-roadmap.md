# Go → Rust 渐进迁移路线图

## 目标

把 `mihomo/` 中适合先迁移的能力逐块收回到 Tauri Rust 后端，遵循同一原则：

1. **先抽离边缘能力**：配置、规则、规则数据、诊断、控制器外围逻辑先迁。
2. **Rust 成为单一路径**：一块能力迁完后，不保留 Go/Rust 双校验或双实现兜底。
3. **Go sidecar 暂时只保留运行时核心**：协议栈、TUN、真实转发链路最后处理。
4. **每一步都能单独 PR / 单独回滚**：避免一次性重写核心导致不可验证。

## 已完成

| 阶段 | 能力 | 状态 | 说明 |
| --- | --- | --- | --- |
| Phase 1 | 配置 Schema 校验 | 完成 | Rust native validator 已替代 `verge-mihomo -t` 校验链路 |
| Phase 2 | 规则解析与基础匹配 | 完成 | `DOMAIN` / `IP-CIDR` / 端口 / `NETWORK` / `MATCH` 等已 Rust 化 |
| Phase 2.5 | 规则校验单一路径 | 完成 | 运行时与配置注入规则都先过 Rust rule engine |
| Phase 3 | `GEOIP` / `GEOSITE` 本地匹配 | 完成 | 支持本地 MMDB / `GeoIP.dat` / `GeoSite.dat` 数据加载 |
| Phase 4A | `IP-ASN` / `SRC-IP-ASN` 本地匹配 | 完成 | PR #15；支持本地 ASN MMDB，缺数据 fail-soft |
| Phase 4B | `RULE-SET` 本地规则集加载 | 完成 | PR #16；第一版支持本地 file provider |
| Phase 4C | 进程 / UID / DSCP / inbound 元数据规则 | 完成 | PR #17-#25；已完成 exact/regex process、UID、DSCP、`IN-TYPE` / `IN-USER` / `IN-NAME` |
| Phase 4D | wildcard / logical / sub-rule 规则 | 完成 | PR #27-#31；`PROCESS-*WILDCARD`、`AND` / `OR` / `NOT` / `SUB-RULE` 与 rule explain |
| Phase 5 | 控制器外围逻辑 Rust 化 | 完成 | PR #31-#37；rule explain、config diff、diagnostics summary、latency planner、node selection planner |
| Phase 6A | DNS 控制面 explain / probe planner | 完成 | PR #45；只做 DNS 配置解释和 probe plan，不接管 Go DNS runtime |
| Phase 6B | 订阅更新控制面 / artifact pipeline | 完成 | PR #46-#71；单一事实链：state source_config → artifact → active_artifact_version → runtime，已消除 legacy profile 写回 |
| Phase 7 | 连接 / 流量 / 内存 / 日志事件路径 Rust 化 | 完成（app-facing path） | PR #72-#79；UI 和托盘不再直连 Mihomo WebSocket，统一经 Rust monitor / Tauri event；Go sidecar 仅作为 Rust 内部 runtime event 来源 |

## 已完成阶段详情

### A. `IP-ASN` / `SRC-IP-ASN` 本地匹配

**状态：已完成（PR #15）。**

**原优先级：最高。复杂度：低。**

这是 Phase 4 中最简单的实现，因为当时代码已经具备三块基础：

- `maxminddb` 依赖已经存在。
- `src-tauri/src/core/ip_intelligence.rs` 已有 `GeoLite2-ASN.mmdb` 查询逻辑。
- `rule_geodata.rs` 已建立 rule engine 外部数据上下文。

#### 范围

把规则：

```yaml
IP-ASN,13335,DIRECT
SRC-IP-ASN,15169,Proxy
```

从“只校验格式”改成“可本地匹配”。

#### 建议实现

新增/扩展 `RuleGeoData`：

```rust
RuleGeoData {
    geoip: Option<GeoIpData>,
    geosite: Option<GeoSiteData>,
    asn: Option<AsnData>,
}
```

新增 `AsnData`：

```rust
AsnData::load_default()
  -> app_home / resources:
       ASN.mmdb
       GeoLite2-ASN.mmdb
```

`ParsedRule` 增加：

```rust
IpAsn {
    asn: u32,
    is_src: bool,
    target: String,
}
```

匹配逻辑：

```rust
IP-ASN:
  ip = meta.dst_ip
  asn_data.lookup(ip) == payload_asn

SRC-IP-ASN:
  ip = meta.src_ip
  asn_data.lookup(ip) == payload_asn
```

#### 数据兼容

需要兼容两类常见 ASN MMDB：

1. `GeoLite2-ASN`
   - 字段：`autonomous_system_number`
   - 字段：`autonomous_system_organization`
2. `DBIP-ASN-Lite (compat=GeoLite2-ASN)`
   - 字段兼容 GeoLite2

如果后续要兼容 `ipinfo generic_asn_free.mmdb`，可追加：

```rust
struct IpInfoAsn {
    asn: String, // e.g. "AS13335"
    name: String,
}
```

#### 测试点

- `IP-ASN` 目标 IP 命中。
- `SRC-IP-ASN` 源 IP 命中。
- 无 ASN 数据时不报错，继续 fallthrough 到下一条规则。
- payload 非数字时校验失败。
- IPv4 / IPv6 都能传入查询路径。

#### 完成标准

- `IP-ASN` / `SRC-IP-ASN` 不再进入 `ExternalData`。
- `test_rule_match` 可在本地 ASN MMDB 存在时返回命中结果。
- Go sidecar 不参与 ASN 规则预览/测试。

---

### B. `RULE-SET` 本地规则集加载

**状态：已完成（PR #16）。**

**原优先级：第二。复杂度：中。**

这一步比 ASN 稍复杂，因为它不只是查一个数据库，而是要加载、缓存、解析一组外部规则文件。

#### 范围

把规则：

```yaml
RULE-SET,private,DIRECT
RULE-SET,reject,REJECT
```

从“格式校验”推进到“可本地匹配”。

#### 需要先确认的现有来源

实现前先梳理这些位置：

- 配置中 `rule-providers` 的结构。
- provider 类型：`file` / `http` / `inline` 是否都要支持。
- provider behavior：`domain` / `ipcidr` / `classical`。
- provider 文件实际落盘目录。
- 当前 Rust config validator 是否已完整读取 `rule-providers`。

#### 建议先做最小版本

第一版只支持本地、已存在的 provider 文件：

```yaml
rule-providers:
  private:
    type: file
    behavior: classical
    path: ./rules/private.yaml
```

不在 Rust rule engine 内实现下载更新；下载/更新仍归现有订阅或 Go sidecar 管线。

#### 建议结构

新增：

```rust
RuleSetData {
    sets: HashMap<String, RuleSetMatcher>,
}

RuleSetMatcher {
    behavior: RuleProviderBehavior,
    engine: RuleEngine,
}
```

加载流程：

```rust
RuleSetData::from_rule_providers(rule_providers)
  -> resolve provider path
  -> parse YAML / text
  -> normalize rules
  -> RuleEngine::from_rules_with_geo_data(...)
```

匹配流程：

```rust
RULE-SET,name,target:
  matcher = rule_set_data.get(name)
  if matcher.match_connection(meta).matched:
      return target
```

注意：`RULE-SET` 的最终策略应使用外层规则的 `target`，不是规则集内部规则的 target。

#### 文件格式兼容

优先支持：

```yaml
payload:
  - DOMAIN-SUFFIX,example.com
  - IP-CIDR,10.0.0.0/8
```

以及纯文本：

```text
DOMAIN-SUFFIX,example.com
IP-CIDR,10.0.0.0/8
```

`classical` 可直接复用 `rule_engine`；`domain` / `ipcidr` 可转成内部规则：

```rust
behavior=domain:
  example.com -> DOMAIN-SUFFIX,example.com,<internal>

behavior=ipcidr:
  10.0.0.0/8 -> IP-CIDR,10.0.0.0/8,<internal>
```

#### 测试点

- 本地 YAML provider 命中 domain。
- 本地 YAML provider 命中 IP CIDR。
- `RULE-SET` 外层 target 覆盖内部 target。
- 缺失 provider 不 panic，继续 fallthrough。
- 循环引用或 RULE-SET 嵌套应拒绝或限制深度。

#### 完成标准

- `RULE-SET` 可在 Rust 规则测试中使用本地 provider。
- 第一版不要求 Rust 下载远程 provider。
- Go sidecar 不参与规则集预览/测试。

---

### C. 下一阶段总路线

#### Phase 4：补齐规则引擎外部数据类型

当前进度：

1. `IP-ASN` / `SRC-IP-ASN`：已完成（PR #15）。
2. `RULE-SET`：已完成（PR #16）。
3. `PROCESS-NAME`：已完成（PR #17）。
4. `PROCESS-PATH`：已完成（PR #18）。
5. `PROCESS-NAME-REGEX`：已完成（PR #19）。
6. `PROCESS-PATH-REGEX`：已完成（PR #20）。
7. `UID`：已完成（PR #21）。
8. `DSCP`：已完成（PR #22）。
9. `IN-TYPE` / `IN-USER` / `IN-NAME`：已完成（PR #23-#25）。
10. `PROCESS-NAME-WILDCARD` / `PROCESS-PATH-WILDCARD`：已完成（PR #27）。
11. `AND` / `OR` / `NOT` / `SUB-RULE`：已完成（PR #31）。

说明：

- ASN 与 RULE-SET 仍属于“数据查表 + 规则复用”，风险低。
- PROCESS/UID/IN-* 开始涉及 OS、进程权限、入口监听器上下文，复杂度会明显上升。
- 当前 Rust 侧只消费 `ConnectionMeta` 已提供的 process / uid / dscp / inbound metadata；不负责 OS 级进程发现或 inbound runtime 采集。
- Phase 4 外部数据类规则已闭环；Phase 5 已继续把控制器外围逻辑迁入 Rust。

#### Phase 5：控制器外围逻辑 Rust 化

当前进度：

1. 规则预览 / 规则解释器：已完成（PR #31）。
2. 配置 diff / explain：已完成（PR #33）。
3. runtime diagnostics 聚合：已完成（PR #34）。
4. latency test 调度层：已完成（PR #35）。
5. 节点选择策略的外层编排：已完成 Rust plan / explain 层（PR #37）。

这类逻辑不碰真实转发链路，适合继续迁。

Phase 5 的删除边界：

- Rust 已接管的控制器外围能力，不再保留前端或 Go 侧同类预览 / explain / 规划兜底入口。
- Go `mihomo/` 中的 rule matching、`URLTest`、provider health check、tunnel scheduler 仍属于真实 runtime / forwarding 数据来源；在 Rust 尚未接管 runtime 前不能删除。
- 每次迁完一个外围能力，应同步删除旧 wrapper / fallback，并在测试里固定调用 Rust Tauri command。

#### Phase 6：DNS 与订阅更新控制面迁移

##### Phase 6A：DNS 解析迁移

当前进度：

1. Rust DNS config explain / probe planner：已完成（PR #45）。
2. Rust DNS resolver runtime：未开始。

已完成范围：

- 读取 runtime YAML 中的 DNS 配置，输出结构化 explain。
- 校验 / 解释 `nameserver`、`fallback`、`proxy-server-nameserver`、`nameserver-policy`、`enhanced-mode`、`fake-ip-range` 等控制面信息。
- 规划 DNS probe / health check 输入。

仍未迁移：

- DNS resolver runtime。
- fake-ip 缓存、fallback-filter、nameserver-policy 的真实运行时行为。
- Go sidecar 的真实 DNS 解析链路。

候选技术：

- `hickory-resolver`
- `hickory-proto`

后续仍建议拆成三步：

1. Rust DNS 配置校验与 explain：已完成。
2. Rust DNS probe / health check planner：已完成控制面 planner，真实 health check 可继续补。
3. Rust DNS resolver runtime：最后做。

不要一上来替换 Go 的 DNS runtime，否则会同时碰缓存、fake-ip、fallback-filter、nameserver-policy。

##### Phase 6B：订阅更新 pipeline 迁移

当前进度：**完成（截至 PR #71）**。

已完成：

1. Payload format detection：已完成（PR #46）。
2. Clash YAML artifact materialization：已完成（PR #47）。
3. Update attempt stage history：已完成（PR #48）。
4. Subscription state reader：已完成（PR #50）。
5. Artifact diagnostics / metadata / content / summary readers：已完成（PR #51-#54）。
6. Artifact cleanup / retention：已完成（PR #55）。
7. Update event timeline：已完成（PR #56）。
8. Transport plan explain：已完成（PR #57）。
9. Legacy profile → typed `SubscriptionSource` read-only view：已完成（PR #58）。
10. `SubscriptionUpdateExecutor` state machine：已完成（PR #59）。
11. Runtime candidate validation + `PublishArtifact` / `active_artifact_version` 切换：已完成（PR #60）。
12. 订阅源配置持久化到 state / orchestration 下沉 / app 层瘦身：已完成（PR #62-#70）。
13. 单一事实链收敛：已完成（PR #71）。移除所有 legacy compatibility projection / snapshot / sync 路径。

当前 pipeline 形态（单一事实链）：

```text
profile command -> state.source_config
state.source_config -> ResolveTransportPlan
  -> FetchPayload
  -> DecodePayload
  -> MaterializeArtifact
  -> GenerateRuntimeConfigCandidate (from artifact)
  -> ValidateRuntimeCandidate (Rust native validator)
  -> PublishArtifact (active_artifact_version)
  -> ActivateRuntime (source_config + active artifact)
  -> EmitFinalResult
```

当前不能删除的边界：

- `profiles.yaml` 仍是用户 profile/source 的 UI 元数据和当前选中记录。
- `CoreManager::update_config_without_restart_with_force(...)` 仍是 runtime activation 入口。
- Go sidecar 仍负责真实 runtime / forwarding。

#### Phase 7：连接统计 / 流量监控 / 日志事件路径

当前进度：**完成 app-facing 单一路径（截至 PR #79）**。

已完成：

1. Rust `ConnectionMetricsAggregator` / `ConnectionMetricsSnapshot` 统一指标模型：已完成（PR #72）。
2. Rust `ConnectionMonitorController` 持续消费 Mihomo `/connections` 事件并发出 `verge://connection-metrics`：已完成（PR #73）。
3. 前端连接页面切到 Rust event path：已完成（PR #74）。
4. 前端 traffic speed 切到 Rust metrics：已完成（PR #75）。
5. 前端 memory usage 切到 Rust metrics：已完成（PR #76）。
6. macOS tray speed 复用 Rust unified metrics，删除 Rust `/traffic` stream adapter：已完成（PR #77）。
7. 删除前端插件层 `ws_traffic` / `ws_memory` / `ws_connections` API：已完成（PR #78）。
8. 日志页面切到 Rust `LogMonitorController` / `verge://core-log`，删除前端 `MihomoWebSocket` / `ws_logs` API：已完成（PR #79）。

当前 app-facing 链路：

```text
metrics:
Mihomo /connections WS
  -> Rust ConnectionMonitorController
  -> ConnectionMetricsAggregator
  -> verge://connection-metrics
  -> frontend connections / traffic / memory
  -> tray speed internal subscriber

logs:
Mihomo /logs WS
  -> Rust LogMonitorController
  -> verge://core-log
  -> frontend logs page
```

已消除的双路：

- 前端不再直接调用 `MihomoWebSocket.connect_connections()`。
- 前端不再直接调用 `MihomoWebSocket.connect_traffic()`。
- 前端不再直接调用 `MihomoWebSocket.connect_memory()`。
- 前端不再直接调用 `MihomoWebSocket.connect_logs()`。
- 托盘速率不再单独连接 Mihomo `/traffic`。
- `tauri-plugin-mihomo` 不再向前端暴露 metrics/logs WebSocket commands。

仍未迁移的底层边界：

- 真实连接、流量、内存、日志数据仍来自 Go sidecar runtime。
- Rust 当前负责 app-facing 聚合、生命周期、事件分发和 API 边界；尚未接管 tunnel / adapter / DNS resolver / protocol runtime。
- `Mihomo::ws_connections()` / `Mihomo::ws_logs()` 仍保留为 Rust 内部桥接，不再是前端 API。

#### Phase 8：协议栈 / TUN / 数据转发

最后处理：

- VMess / VLESS / Trojan / TUIC / Hysteria
- TUN
- transparent proxy
- tunnel
- adapter outbound/inbound

这是最大块，不建议近期开始。

## 每个迁移 PR 的固定检查清单

1. 新能力进入 Rust 单一路径。
2. Go 侧同类校验/预览/辅助路径不再兜底。
3. 缺少外部数据时必须 fail-soft，不能 panic。
4. 新增 focused unit tests。
5. `test_rule_match` 或对应 Tauri command 能走 Rust 路径验证。
6. PR 描述里写清楚：
   - 已迁移能力
   - 未迁移能力
   - 数据文件位置
   - 本地验证命令

## 不建议现在做的事

- 不要直接替换 Go sidecar。
- 不要先碰协议栈。
- 不要同时迁 DNS runtime 和 RULE-SET。
- 不要保留 Go/Rust 两条规则校验长期并行。
- 不要为了让测试通过硬编码常见 ASN / 域名 / 国家码。

## 推荐的下一个实际开发 PR

按当前状态，Phase 7 的 app-facing 直连 WebSocket 双路已经清掉；下一张实现 PR 建议回到 Phase 6A 未完成项，开始 DNS runtime 接管的最小可验证切片：

```text
feat(dns): introduce Rust resolver runtime skeleton
```

建议范围只包含：

- Rust DNS runtime trait / controller skeleton。
- 从现有 Clash DNS config 构建 resolver plan。
- 先支持普通 nameserver 查询和 timeout / retry / metrics。
- 保持 fake-ip、fallback-filter、nameserver-policy 只做 plan/explain，不在第一刀替换。

不包含：

- 协议栈 / TUN / tunnel。
- adapter outbound/inbound。
- 一次性替换 Go DNS runtime。
