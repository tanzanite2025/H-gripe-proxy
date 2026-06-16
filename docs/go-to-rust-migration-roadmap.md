# Go → Rust 渐进迁移路线图

## 目标

把 `mihomo/` 中适合先迁移的能力逐块收回到 Tauri Rust 后端，遵循同一原则：

1. **先抽离边缘能力**：配置、规则、规则数据、诊断、控制器外围逻辑先迁。
2. **Rust 成为单一路径**：一块能力迁完后，不保留 Go/Rust 双校验或双实现兜底。
3. **Go sidecar 暂时只保留运行时核心**：协议栈、TUN、真实转发链路最后处理。
4. **每一步都能单独 PR / 单独回滚**：避免一次性重写核心导致不可验证。

同时迁移路线必须兼顾最终产品形态：项目不只是把 Go 内核逐步替换成 Rust，也要演进成 **应用级代理编排平台**：

```text
App registry / software profile
  -> node pool / DNS profile / security profile
  -> Rust routing + runtime plan
  -> Go Mihomo runtime bridge now
  -> Rust-owned runtime later
```

因此 Go → Rust 迁移不能只按“替换底层模块”线性推进。凡是会影响最终应用管理、节点池、自定义出口策略的数据模型，都应在 Rust 控制面阶段同步预埋，避免底层迁完后再重写一遍 profile / policy / runtime plan。

## 并行主线：应用级代理编排

最终主线是 **软件内部添加软件，并为每个软件绑定自定义节点池和运行策略**。这条主线应与 Go → Rust 迁移并行推进，但每一步仍保持单一事实链。

### 可以与 Go → Rust 同时做的部分

这些能力属于 Rust 控制面 / 数据模型 / 诊断层，不依赖完整 Rust 协议栈，适合现在同步建设：

1. **App registry / 软件资产模型**
   - `app_id`
   - executable path / app bundle id
   - launch args / working directory / env
   - process matcher
   - platform-specific metadata

2. **Node pool / 节点池模型**
   - pool id / name / tags
   - region / protocol / cost / purpose
   - latency / availability / failover constraints
   - pool-level health summary

3. **App policy binding**
   - app → node pool
   - app → DNS profile
   - app → security profile
   - app → routing intent（direct / proxy / reject / auto / fallback）

4. **Rust runtime plan / explain**
   - 把 app + node pool + DNS + security policy 编译成结构化 plan。
   - 当前阶段可继续输出到 Mihomo runtime config。
   - 后续 Rust 接管 tunnel / outbound 后，复用同一个 plan，不改变上层模型。

5. **App-scoped observability**
   - per-app connection view
   - per-app exit IP verification
   - per-app DNS leak / proxy leak diagnosis
   - node pool health 与实际出口一致性检测

6. **App session lifecycle**
   - Rust 记录 app session。
   - 启动前解析 policy plan。
   - 运行时订阅连接 / 日志 / DNS 诊断。
   - 退出后保留诊断结果和策略表现。

### 不应提前做的部分

这些能力依赖底层 runtime / OS 网络接管，不宜在 Go sidecar 尚未替换前硬做，否则会产生新的双路：

- 自研 outbound / inbound 协议栈。
- 完整 TUN / transparent proxy 替换。
- 强 per-app 网络隔离或 sandbox。
- 直接绕过 Mihomo runtime 的真实流量转发。
- 与 Go sidecar 并行维护第二套路由执行器。

### 单一事实链要求

应用级编排主线也必须遵循同一事实链：

```text
app registry
  -> app policy binding
  -> node pool / DNS / security profiles
  -> Rust runtime plan
  -> generated runtime config / active runtime
  -> observed app session state
```

禁止：

- UI 一套 app/pool 配置，runtime 另一套临时配置。
- app policy 直接写入 Mihomo YAML，但 Rust state 不知道。
- 节点池选择在前端临时计算。
- Go runtime 与 Rust runtime plan 长期并行竞争决策权。

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
| Phase 6A.1 | DNS resolver runtime skeleton / controlled probe | 完成（opt-in probe path） | PR #83/#93/#94；Rust `DnsResolverPlan` / hickory query controller / per-nameserver controlled probe UI 已落地，默认 DNS runtime 与 fake-ip / fallback-filter / nameserver-policy 仍 plan-only |
| Phase 6B | 订阅更新控制面 / artifact pipeline | 完成 | PR #46-#71；单一事实链：state source_config → artifact → active_artifact_version → runtime，已消除 legacy profile 写回 |
| Phase 7 | 连接 / 流量 / 内存 / 日志事件路径 Rust 化 | 完成（app-facing path） | PR #72-#79；UI 和托盘不再直连 Mihomo WebSocket，统一经 Rust monitor / Tauri event；Go sidecar 仅作为 Rust 内部 runtime event 来源 |
| Phase 7.5 | 应用级代理编排控制面 | 进行中（DNS default runtime execution 后验证前置） | PR #82/#84-#91/#95-#132、Batch J-K（PR #134/#135）、Batch L-Q（PR #136-#140、本批次）；AppRuntimeStateDocument、RuntimePlan、Mihomo projection、diagnostics、session observation/evaluation/leak planning、CRUD/form 管理、聚合诊断动作、readiness 检查、staged artifact preflight、active marker、marker rollback、显式 opt-in runtime candidate apply guard、runtime apply audit / observed verification、默认 DNS runtime readiness gate / shadow evidence / opt-in switch guard / executor preflight / execution guard / limited opt-in execution / post-execution observed verification 与 rollback drill 已进入 Rust 单一路径；下一步评估是否允许更大范围 opt-in execution |

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
2. Rust DNS resolver runtime skeleton：已完成（PR #83）。
3. Rust DNS controlled runtime probe：已完成（PR #93/#94）。
4. Rust DNS resolver default runtime：未接管。

已完成范围：

- 读取 runtime YAML 中的 DNS 配置，输出结构化 explain。
- 校验 / 解释 `nameserver`、`fallback`、`proxy-server-nameserver`、`nameserver-policy`、`enhanced-mode`、`fake-ip-range` 等控制面信息。
- 规划 DNS probe / health check 输入。
- 从 Clash DNS 配置构建 Rust `DnsResolverPlan`，标记 runtime-supported nameserver。
- 提供 hickory-backed query controller / `dns_runtime_query` skeleton，用于受控查询、timeout / retry / metrics 试运行。
- 提供 `dns_controlled_runtime_probe` / `dnsControlledRuntimeProbe(...)`，按 nameserver 输出 supported / healthy / latency / provider / warning summary。
- DNS stats UI 可从当前 Rust-observed runtime DNS 列表触发 controlled probe；该入口只诊断 Rust resolver 支持能力，不写 Mihomo runtime。

仍未迁移：

- 默认 DNS resolver runtime 切换。
- fake-ip 缓存、fallback-filter、nameserver-policy 的真实运行时行为；当前只在 `DnsResolverRuntimeProjection` 中声明 plan-only 边界。
- Go sidecar 的真实 DNS 解析链路；应用默认流量仍不走 Rust DNS runtime。

候选技术：

- `hickory-resolver`：已用于 skeleton 查询控制器。
- `hickory-proto`：已用于协议 / nameserver target 解析。

后续仍建议拆成三步：

1. Rust DNS 配置校验与 explain：已完成。
2. Rust DNS probe / health check planner：已完成控制面 planner。
3. Rust DNS resolver runtime skeleton：已完成。
4. Rust DNS controlled runtime probe：已完成 opt-in 查询、provider health、metrics 和失败归因。
5. Rust DNS default runtime：最后做。

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

#### Phase 7.5：应用级代理编排控制面

当前进度：**完成 planning / session / CRUD / form / observability / readiness / staged projection artifact path / active marker rollback / explicit runtime apply guard / runtime apply audit 与 observed verification / default DNS runtime readiness gate / shadow evidence / opt-in switch guard / executor preflight / execution guard / limited opt-in execution（PR #140），并完成 app-runtime backend 第二轮拆分**。

目标不是新增一个普通“应用列表”，而是为最终 app-centric proxy orchestration 建立 Rust-owned 数据链：

```text
AppRegistry
  -> AppPolicyBinding
  -> NodePool / DnsProfile / SecurityProfile
  -> RuntimePlan
  -> staged AppRuntimeProjectionArtifact now
  -> controlled activation gate later
  -> Rust runtime execution after that
```

已完成：

1. **App runtime state / plan（PR #82）**
   - `AppRuntimeStateDocument` 作为唯一事实源。
   - `AppRegistryEntry` / `NodePool` / `DnsProfile` / `SecurityProfile` / `AppPolicyBinding` 已落盘到 Rust state。
   - `explain_app_runtime_plan` 生成 planning-only `RuntimePlan`，不修改 Mihomo runtime。

2. **DNS profile planning（PR #84）**
   - app policy 可绑定 DNS profile。
   - DNS profile 通过 `DnsResolverPlan` 暴露 supported / unsupported nameserver 与 runtime projection 边界。

3. **Plan-to-Mihomo projection（PR #85）**
   - Rust plan 可生成 Mihomo proxy-group / rule / DNS YAML patch。
   - `mutatesRuntime=false`，projection 只作为执行候选产物，不成为事实源。

4. **App runtime diagnostics（PR #86）**
   - 统一检查 app / policy / node pool / DNS / security / projection readiness。
   - 诊断仍保持 planning-only，不启动软件、不接管网络。

5. **App session lifecycle（PR #87）**
   - Rust 记录 app session。
   - session snapshot 固化 plan status、diagnostics summary、projected rules / proxy-groups。

6. **App-scoped observability（PR #88-#90）**
   - 复用 Phase 7 metrics/logs 事件路径。
   - 从 `ConnectionMetricsSnapshot` 记录 session observation。
   - 使用 projected rules / proxy-groups 匹配 connection attribution candidates。
   - `evaluate_app_runtime_session` 汇总 matched / mismatched / unattributed / stale observation。

7. **Leak verification planning（PR #91）**
   - 基于 session observation 与 plan projection 检查 proxy / DNS / exit / node-pool 一致性。
   - 仍是规划和观测归因，不做 live exit probe，也不声称强 per-app isolation。

8. **App runtime planning / session UI（PR #95-#97）**
   - “高级功能 → 应用编排”面板读取 `AppRuntimeStateDocument`，展示 app / node pool / DNS profile / security profile / binding inventory。
   - 可对已注册 app 触发 `diagnoseAppRuntime` 与 `projectAppRuntimePlanToMihomo`，展示 plan / diagnostics / YAML patch，仍保持 `mutatesRuntime=false`。
   - 可启动 app runtime session、记录 connection metrics snapshot、评估 attribution、检查 leak dimensions，并将 session 标记为 completed / blocked / failed。
   - 这些 UI 操作只写 Rust app-runtime state / session record，不直接生成前端临时 Mihomo rules，也不启动或隔离第三方应用。

9. **App runtime accelerated UI batches（PR #100-#104）**
   - PR #100：在同一“应用编排”面板中提供 app / node pool / DNS profile / security profile / policy binding 的 JSON CRUD，以及批量 import / export。
   - PR #101：从 app policy binding 定位 DNS profile，并 opt-in 调用 `dns_controlled_runtime_probe` 展示 runtime-supported / healthy / failed / warning summary。
   - PR #102：展开 session observability，展示 observation timeline、traffic totals、attribution candidates、evaluation details 与 leak dimension facts/warnings。
   - PR #103/#104：提供 app → binding → node / DNS / security → session 的 overview matrix，并标出 missing binding、disabled binding、缺失 profile 引用等 state issue。
   - 这些加速批次仍只围绕 Rust `AppRuntimeStateDocument` / session reports / controlled probe，不切默认 DNS runtime，不注入或修改真实 Mihomo 转发。

10. **App runtime form 管理面（PR #106-#110）**
    - Policy binding、app registry、node pool、DNS profile、security profile 的高频字段已从 JSON editor 提升为表单。
    - 保存仍走 Rust upsert command，表单不直接生成 Mihomo 规则，也不绕过 `AppRuntimeStateDocument`。

11. **App runtime 面板结构拆分（PR #113-#120）**
    - `app-runtime-planning-panel.tsx` 从约 2937 行拆到约 1453 行。
    - 已拆出 utils、overview、aggregate diagnostics、session、resource manager、security/DNS/node/app/binding forms、planning result panel。
    - 这是后续按中等批次继续推进的前置整理，不改变 Rust command / runtime boundary。

12. **聚合诊断动作与 readiness 检查（PR #111-#112/#121-#122）**
    - 聚合 overview state issue、planning diagnostics、DNS controlled probe、runtime boundary，并生成待处理动作。
    - 待处理动作可定位 state issue 到 overview / resource editor，或触发现有 planning diagnostics / DNS controlled probe。
    - 新增 selected-app readiness action，串联 `diagnoseAppRuntime` + `projectAppRuntimePlanToMihomo`，并在绑定 DNS profile 时 opt-in 运行 `dns_controlled_runtime_probe`。
    - readiness 只填充现有 diagnostics / projection / DNS probe UI state，不修改 Mihomo runtime，不启动或隔离第三方应用。

13. **Runtime projection artifact gate（PR #124）**
    - 新增 `AppRuntimeProjectionArtifact` 与 `build_app_runtime_projection_artifact` command。
    - Artifact 从 Rust `AppRuntimeStateDocument -> RuntimePlan -> AppRuntimeMihomoProjection -> diagnostics` 生成，包含 app / binding / node pool / DNS / security 引用、checksum、generatedAt、validation report。
    - 明确 `activationMode=staged`、`mutatesRuntime=false`；validation 复用 diagnostics gate，并额外检查 YAML patch parse、rule projection、runtime boundary。
    - 前端新增 `生成 artifact` 动作与 artifact status / checksum / validation checks 展示，不切 active profile，不 reload Mihomo。

14. **Projection artifact 持久化（PR #125）**
    - Tauri command 生成 artifact 后写入 `app-runtime/artifacts/<artifact-id>/artifact.yaml`，并返回 `storagePath`。
    - 新增 app-runtime artifact 目录 helper 与 safe path segment，避免 app/artifact id 直接成为任意 filesystem path。
    - UI 展示 artifact 存储路径，使候选执行产物可审计、可 review，再进入后续 controlled activation。
    - 仍不执行 runtime activation；持久化 artifact 只是把 execution candidate 固化为 Rust-owned audit record。

15. **Controlled activation preflight / active marker（PR #127/#128）**
    - 新增 activation preflight command，只从已持久化 `AppRuntimeProjectionArtifact` 读取，校验 artifact id、checksum、validation status、`activationMode=staged` 与 `mutatesRuntime=false`。
    - preflight 首批保留 executor guard 为 blocked，不 reload / restart Mihomo，不写 active profile，不把 YAML patch 当作事实源。
    - 新增 active projection marker，记录当前 staged artifact、checksum、storage path 与 rollback metadata；该 marker 只写 Rust `AppRuntimeStateDocument.activeProjection`。
    - UI 可显式触发 preflight 与标记 active，并展示 active marker / rollback 元数据，仍保持 `mutatesRuntime=false`。

16. **Active projection rollback guard（PR #130）**
    - 新增 active projection marker rollback command，只回滚 Rust `AppRuntimeStateDocument.activeProjection`。
    - 若 previous artifact 存在，rollback 会重新读取持久化 artifact 并校验 checksum、validation status 与 staged runtime boundary。
    - 若 previous artifact 为空，rollback 清空 active marker；不会 reload / restart Mihomo，也不会写 active profile。
    - UI 仅提供显式用户触发的 marker rollback 动作。

17. **Explicit runtime candidate apply guard（PR #131）**
    - 新增显式用户触发的 runtime candidate apply command，从已持久化 artifact 读取并复用 checksum / validation / staged boundary gate。
    - 将 projection YAML patch 组合为临时 profile merge candidate，通过现有 `CoreManager::update_config_without_restart_with_force(...)` 入口应用；不持久修改 `profiles.yaml`，不把前端临时 YAML 当事实源。
    - active marker 标记 `mutatesRuntime=true`，并要求 rollback 先恢复 runtime，再恢复 runtime apply 前的 marker-only state。
    - 当前仅覆盖 profile merge candidate 这条显式 opt-in 路径；仍不迁默认 DNS runtime / TUN / protocol 数据面。

18. **App runtime backend second split（PR #132）**
    - `app_runtime.rs` 从约 3039 行降到约 993 行。
    - `app_runtime/projection.rs` 承接 Mihomo projection、artifact、activation、runtime apply 与 rollback 逻辑。
    - `app_runtime/sessions.rs` 承接 session lifecycle、observation、evaluation 与 leak check 逻辑。
    - 只做维护性拆分，公共 `core::app_runtime` re-export API 保持不变。

删除边界：

- 不新增前端直接操作 Mihomo proxy-group 的 app 级旁路。
- 不新增第二套与 Rust `RuntimePlan` 并行的 routing decision。
- 不在 Rust outbound 未完成前声称已经实现强 per-app isolation。
- 不让 `yaml_patch`、session observation 或 leak report 反向成为 app / node pool / DNS / security policy 的事实源。

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
6. 如果 PR 涉及 app / node pool / policy，必须说明：
   - Rust state 中的唯一事实源。
   - runtime projection 是否只是执行产物。
   - 后续切换到 Rust runtime 时是否可复用同一模型。
7. PR 描述里写清楚：
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
- 不要在 Rust state 之外新增 app / node pool 的临时事实源。
- 不要让前端直接生成 app 级 Mihomo 规则作为长期方案。

## 加速执行策略

前一轮 PR 进度偏慢的主要原因不是技术阻塞，而是切片过细。PR #100-#132 与后续 Batch J/K/L/M/N/O/P（PR #134-#140）已经把 app-runtime control-plane / diagnostics UI 的基础能力补齐，完成主面板与后端 app-runtime 第二轮拆分，并把 Rust-owned plan 推进到 **可验证、可审计的 staged runtime projection artifact**、activation preflight guard、active artifact marker、marker rollback、显式 opt-in runtime candidate apply guard、runtime apply audit / observed verification、默认 DNS runtime readiness gate、shadow evidence、opt-in switch guard、executor dry-run preflight、execution guard 与 limited opt-in execution。后续不能继续停留在零散 UI 增强，应回到 Go → Rust 主线：再评估执行后 observed verification / rollback drill、TUN / protocol runtime 边界。

### 可以加快做的部分

这些能力都停留在 Rust 控制面、state、diagnostics 或 frontend surface，不直接碰真实流量转发，可合并成较大的功能 PR：

| 优先级 | 可加速方向 | 建议合并方式 | 原因 |
| --- | --- | --- | --- |
| P0 | Runtime projection artifact / diff / validation gate | 已完成 staged artifact + 持久化（PR #124/#125） | 已从 planning-only 推进到可审计 execution candidate；仍未切 active runtime |
| P0 | Controlled activation / runtime apply guard | 已完成 preflight + active marker（PR #127/#128）、rollback guard（PR #130）与 explicit runtime candidate apply guard（PR #131） | 只允许从持久化 artifact 出发，先做 preflight/guard，再做显式 opt-in runtime candidate，不直接扩大到 TUN/DNS/protocol runtime |
| P0 | App diagnostics 串联 DNS probe / readiness | 已完成聚合诊断动作和 readiness action（PR #111-#112/#121-#122） | 已有 `dns_controlled_runtime_probe`，仍是 opt-in，不切默认 DNS runtime |
| P1 | App runtime CRUD / form 管理面 | 已完成 JSON CRUD、import/export 与常用字段表单（PR #100/#106-#110） | 后续只补必要字段，避免继续做零散 UI |
| P1 | 面板 / 后端结构维护 | 已完成主面板拆分（PR #113-#120）与 app-runtime backend 第二轮拆分（PR #132） | 后续功能必须落在已拆分组件 / modules 中，避免重新形成巨型文件 |
| P2 | Demo / seed import-export | 可在 activation preflight 后再做 | 只有当 artifact / validation / preflight chain 稳定后，样例数据才真正服务主线验证 |

### 不应加速硬推的部分

这些能力会进入真实 runtime / OS 网络边界，应继续小步、强验证：

| 高风险方向 | 为什么不能快 |
| --- | --- |
| 默认 DNS resolver runtime 切换 | 会同时影响 cache、fake-ip、fallback-filter、nameserver-policy 和用户默认解析路径 |
| TUN / transparent proxy / tunnel | 涉及系统网络接管、权限、平台差异和回滚复杂度 |
| 自研 outbound / inbound 协议栈 | 会替换 Mihomo 核心数据面，是最大风险块 |
| 强 per-app network isolation / sandbox | 依赖 OS 网络隔离能力，不能只靠 control-plane 文案声明完成 |
| 前端直接写 Mihomo app rules | 会破坏 Rust `AppRuntimeStateDocument -> RuntimePlan -> projection` 单一事实链 |

### 新 PR 节奏

后续开发默认按这个节奏推进：

1. **控制面 / UI / diagnostics：合并成中等 PR。** 同一页面、同一 state、同一风险边界的改动尽量一次完成。
2. **真实 runtime：继续拆小 PR。** 任何会改变默认 DNS、TUN、adapter、protocol 或 active forwarding 的改动都必须单独 PR。
3. **文档不再跟每个小切片同步。** 路线图只在一个批次完成后更新，PR 描述记录本批次边界即可。
4. **每个加速 PR 仍必须保留边界声明。** PR body 写清楚 `mutatesRuntime=false` / opt-in / plan-only，不用小 PR 换安全感。

## 推荐的下一个实际开发批次

从提交记录看，Batch D / E、结构拆分、Batch F staged artifact gate、Batch G activation preflight / active marker、Batch H active projection rollback guard、Batch I explicit runtime candidate apply guard、Batch J runtime apply audit / observed verification、Batch K default DNS runtime readiness gate、Batch L default DNS runtime shadow evidence、Batch M default DNS runtime opt-in switch guard、Batch N default DNS runtime opt-in executor preflight、Batch O default DNS runtime opt-in execution guard、Batch P default DNS runtime limited opt-in execution（PR #140）都已完成：Rust-owned app-runtime state 可编辑、可表单化管理、可导入导出、可做绑定 DNS controlled probe、可查看 session 细节、可在 overview matrix 中定位断链，可一键 readiness，可生成/持久化 staged projection artifact，并可从持久化 artifact 做 activation preflight、active marker、marker rollback、显式 runtime candidate apply、runtime apply audit、只读运行态验证、默认 DNS runtime readiness/blocker、shadow evidence、opt-in switch guard、executor preflight、execution guard 与 limited execution 评估。

下一步仍不应直接切 TUN 或协议栈替换；这些会扩大到真实数据面。默认 DNS runtime 若继续推进，应基于 **Batch Q：Default DNS runtime post-execution observed verification** 的执行后 observed verification、rollback drill 与 failure audit 结果，评估是否允许更大范围 opt-in execution。

### Batch F：Runtime projection artifact / diff / validation gate（已完成 PR #124/#125）

已完成 PR：

```text
feat(app-runtime): add projection artifact validation gate
feat(app-runtime): persist projection artifacts
```

结果：把原本只显示在 UI 的 `yamlPatch` 升级为 Rust-owned、可审计、可验证的 staged projection artifact，并持久化到 app-runtime artifact 目录；仍不自动修改 Mihomo runtime。

已落地范围：

- 新增 Rust command：对 selected app 生成 `AppRuntimeProjectionArtifact`。
- Artifact 包含：
  - app id / binding id / referenced node pool / DNS profile / security profile。
  - `RuntimePlan`、`AppRuntimeMihomoProjection`、projection checksum、生成时间。
  - `mutatesRuntime=false` / `activationMode=staged` 边界字段。
- 复用现有 Rust diagnostics gate，并新增 artifact-level dry-run validation：
  - diagnostics gate。
  - YAML patch parse。
  - rule projection。
  - `mutatesRuntime=false` runtime boundary。
- Tauri command 生成后持久化到 `app-runtime/artifacts/<artifact-id>/artifact.yaml`，返回 `storagePath`。
- UI 展示 artifact readiness、checksum、activation mode、validation blockers 与 storage path。

不包含：

- 不切换 active profile。
- 不调用 Mihomo reload / restart。
- 不把 `yamlPatch` 直接写入用户配置作为事实源。
- 不迁默认 DNS resolver runtime。
- 不碰 TUN / transparent proxy / tunnel / adapter outbound/inbound / protocol runtime。

### Batch G：Controlled activation gate（已完成 PR #127/#128）

Batch F 已经把 artifact / validation / audit record 串起来。Batch G 进入 controlled activation，但首批只做 preflight / guard：

```text
feat(app-runtime): add opt-in projection activation gate
```

建议边界：

- activation 必须从 Rust artifact 出发，不能从前端临时 YAML 出发。
- activation 必须先读取已持久化 artifact，并校验 checksum / validation status / `mutatesRuntime=false` 边界。
- activation 必须走现有 profile / artifact / validation 单一路径，并保留 rollback metadata。
- 首版只做显式用户触发，不随页面打开或 readiness 自动执行。
- 首版只桥接到现有 Mihomo runtime；不声称 Rust 已经接管协议栈或强 per-app isolation。
- 若 preflight 发现 active profile、runtime candidate 写入、rollback metadata 或 validation hook 不完整，应先返回 blocker，不应直接调用 Mihomo reload。

Batch G 的目标不是“一步实现 per-app runtime isolation”，而是建立受控激活门：只有 Rust-owned artifact 通过 preflight，才允许后续显式 opt-in activation PR 进入真实 runtime mutation。

已落地范围：

- `preflight_app_runtime_projection_activation` 从持久化 artifact 读取，不接受前端临时 YAML。
- preflight 校验 artifact id / checksum / validation status / staged runtime boundary。
- executor guard 在首批保持 blocked，明确不 reload / restart Mihomo。
- `activate_app_runtime_projection_artifact` 只写 `AppRuntimeStateDocument.activeProjection` marker，并记录 rollback metadata。
- UI 可显式触发 preflight 与标记 active，仍不修改 active profile 或 Mihomo runtime。

### Batch H：Active projection rollback guard（已完成 PR #130）

Batch G 已记录 rollback metadata，但还缺少显式 rollback action。本批次补齐 marker 级回滚，继续保持 no runtime mutation：

```text
feat(app-runtime): add active projection rollback guard
```

建议边界：

- rollback 只作用于 Rust `AppRuntimeStateDocument.activeProjection` marker。
- 若 active marker 没有 previous artifact，rollback 清空 active marker。
- 若存在 previous artifact，必须重新读取持久化 artifact 并校验 checksum / validation / staged boundary。
- rollback 必须防止 active marker 在操作期间被其他写入替换。
- UI 只能由用户显式触发 rollback，不随页面加载自动执行。
- 仍不 reload / restart Mihomo，不写 active profile，不把 projection YAML patch 作为事实源。

### Batch I：Explicit runtime candidate apply guard（已完成 PR #131）

Batch H 已能恢复 active marker。本批次开始进入真实 runtime 边界，但只走已有配置生成入口，仍保持用户显式触发与可回滚：

```text
feat(app-runtime): add explicit runtime candidate apply guard
```

建议边界：

- apply 必须从持久化 artifact 读取，并校验 artifact id / checksum / validation status / staged boundary。
- apply 只创建临时 profile merge candidate，并通过 `CoreManager::update_config_without_restart_with_force(...)` 应用；不持久修改 `profiles.yaml`。
- apply 成功后 active marker 必须记录 `mutatesRuntime=true` 和 runtime rollback strategy。
- rollback 遇到 `mutatesRuntime=true` 时必须先恢复 runtime，再恢复 runtime apply 前的 marker-only state。
- 仍不迁默认 DNS runtime、TUN、transparent proxy、adapter outbound/inbound 或协议栈。

已落地范围：

- 新增 `apply_app_runtime_projection_artifact_to_runtime` command。
- apply 必须匹配当前 active projection marker，且 artifact id / checksum / validation / staged boundary 全部通过。
- 通过临时 profile merge candidate 调用 `CoreManager::update_config_without_restart_with_force(...)`，不持久修改 `profiles.yaml`。
- apply 成功后 active marker 记录 `activationKind=runtime_profile_merge` 与 `mutatesRuntime=true`。
- rollback 遇到 runtime-mutating marker 时先恢复 runtime，再恢复 runtime apply 前的 marker-only state。
- UI 增加显式 `应用 runtime` 按钮；只有当前 active artifact 且未 mutatesRuntime 时可触发。

### Batch J：Runtime apply audit / observed runtime verification（本批次，小步做）

Batch I 已允许显式 opt-in runtime mutation，但当前闭环仍主要停留在“命令成功 / active marker 已变更”。下一批应补齐 audit 与 observed verification，避免进入 DNS/TUN/protocol 前缺少运行态证据：

```text
feat(app-runtime): add runtime apply audit verification
```

建议边界：

- apply 成功后生成 Rust-owned audit record / report，至少包含：
  - artifact id / checksum / activation kind / mutation timestamp。
  - runtime config validation outcome。
  - 临时 candidate merge 摘要（proxy-groups / rules count、profile item identity，不记录敏感配置）。
  - rollback strategy 与 previous marker 摘要。
- 新增 read/list command 读取最近 runtime apply audit，不从前端临时状态推断。
- 新增 observed runtime verification：
  - 从当前 Rust monitor / controller 可见数据检查 active projection 的 proxy-group / rule 是否能在 runtime config 或 controller surface 中被观察到。
  - 输出 `verified / degraded / blocked`，并说明缺少哪些 runtime evidence。
  - 只读 controller/runtime surface，不再次写 profile、不自动 reload。
- UI 在 active marker 下展示 apply audit 与 verification result，使用户能判断“已应用”是否真的进入 Mihomo runtime。
- rollback 后 audit 应能标记该 runtime mutation 已被 rollback / superseded，避免误以为仍生效。

不包含：

- 不切默认 DNS resolver runtime。
- 不接管 fake-ip cache / fallback-filter / nameserver-policy。
- 不碰 TUN / transparent proxy / tunnel / adapter outbound/inbound / protocol runtime。
- 不把 observed runtime config 反向写回 `AppRuntimeStateDocument` 作为事实源；事实源仍是 Rust app-runtime state + artifact。

完成标准：

- Runtime apply 有可持久化 / 可查询的 audit record。
- UI 能展示 active projection 的 latest apply audit 与 observed verification。
- verification 缺少 runtime evidence 时 fail-soft 返回 degraded / blocked，不 panic、不自动修复。
- rollback 后 audit 状态可区分 active / rolled back / superseded。

已落地范围：

- `AppRuntimeStateDocument.runtimeApplyAudits` 记录 runtime apply audit。
- apply 成功后记录 artifact/checksum/activation kind、validation outcome、candidate profile item、proxy-group/rule count、previous marker 与 rollback strategy。
- 新增 `list_app_runtime_projection_runtime_apply_audits` 读取审计记录。
- 新增 `verify_app_runtime_projection_runtime_apply`，只读当前 runtime config，检查 active marker、latest audit、runtime config availability、projected proxy-groups 与 rules 是否可观察。
- verification 结果写回 latest audit 的最近验证状态 / reason / timestamp。
- rollback runtime-mutating marker 时把对应 active audit 标记为 `rolledBack`；后续新 apply 可把旧 active audit 标记为 `superseded`。
- UI 在 projection artifact 面板展示 latest apply audit，并提供显式 `验证 runtime` 按钮展示 observed verification summary。

### Batch K：Default DNS runtime readiness gate（本批次，评估后再做）

Batch J 之后，app-runtime 的 opt-in runtime mutation 已有审计和只读运行态证据。下一步可以开始评估默认 DNS runtime 切换，但首批仍应是 readiness / preflight gate，而不是直接替换 Go DNS runtime：

```text
feat(dns-runtime): add default resolver readiness gate
```

建议边界：

- 只从现有 `DnsResolverPlan` / controlled probe / health summary 出发，不新增第二套 DNS 配置事实源。
- 新增默认 DNS runtime readiness report，汇总：
  - 当前 profile DNS 配置是否能生成 Rust resolver plan。
  - nameserver / fallback / proxy-server-nameserver 的 runtime-supported 覆盖率。
  - controlled probe 最近健康状态与失败归因。
  - fake-ip / fallback-filter / nameserver-policy 仍未接管的 blocker。
- UI 只展示 readiness / blocker，不自动切默认 DNS runtime。
- 不改变用户默认解析路径，不写 active profile，不 reload Mihomo。

不包含：

- 不真正启用 Rust default DNS resolver。
- 不接管 fake-ip cache / fallback-filter / nameserver-policy。
- 不碰 TUN、transparent proxy、adapter outbound/inbound 或协议栈。

已落地范围：

- 新增 `dns_default_runtime_readiness` command，默认只读当前 runtime config，也可显式传入 YAML。
- Readiness report 复用 `DnsResolverPlan`，检查 resolver plan、nameserver runtime support coverage、controlled probe evidence、fake-ip / fallback-filter / nameserver-policy blocker。
- 输出 `ready / degraded / blocked`、summary、checks、blockers、warnings 与 facts。
- DNS runtime stats UI 新增默认 DNS runtime readiness 面板；可复用同页最新 controlled probe report 作为健康证据。
- 继续保持只读：不修改 DNS config、不写 active profile、不 reload Mihomo、不切默认 DNS runtime。

### Batch L：Default DNS runtime shadow evidence（本批次，小步做）

Batch K 只回答“是否具备 readiness”。下一批仍不直接切默认 DNS runtime，而是收集 shadow evidence：同一 test domain / nameserver set 同时走 Rust resolver 与现有 runtime/系统可见路径，比较结果、延迟和失败归因。

```text
feat(dns-runtime): add default resolver shadow evidence
```

建议边界：

- 只读 shadow query / report，不接管默认解析路径。
- 输入必须来自当前 runtime DNS config 或已持久化 DNS profile，不从前端临时编辑状态推断事实源。
- 记录 Rust resolver result、Mihomo/controller-observed 或系统 resolver result、latency、error、provider attribution 与 mismatch reason。
- 若 fake-ip / fallback-filter / nameserver-policy blocker 仍存在，shadow report 必须显式标记“无法证明可替换默认 runtime”。
- UI 只展示 evidence 与 diff，不提供启用默认 DNS runtime 按钮。

不包含：

- 不启用 Rust default DNS resolver。
- 不替换 fake-ip cache / fallback-filter / nameserver-policy。
- 不碰 TUN、transparent proxy、adapter outbound/inbound 或协议栈。

已落地范围：

- 新增 `dns_default_runtime_shadow_evidence` command，默认只读当前 runtime DNS config，也可显式传入 YAML。
- Shadow report 复用 Batch K readiness gate；若 readiness 不是 ready，明确返回 blocked，表示 shadow evidence 不能证明可切换默认 runtime。
- 对同一 test domain 采集 Rust resolver query report 与系统 resolver query result，输出 IP match、latency delta、attempted Rust target 与 mismatch reason。
- DNS runtime stats UI 新增 shadow evidence 面板，只展示 evidence / diff / blocker，不提供启用默认 DNS runtime 按钮。
- 继续保持只读：不修改 DNS config、不写 active profile、不 reload Mihomo、不切默认 DNS runtime。

### Batch M：Default DNS runtime opt-in switch guard（本批次，需谨慎）

Batch K/L 已提供 readiness 与 shadow evidence。若继续向默认 DNS runtime 迁移推进，下一批仍不应自动切换，而是做显式 opt-in switch guard：

```text
feat(dns-runtime): add default resolver opt-in switch guard
```

建议边界：

- switch 必须由用户显式触发，不能由页面加载、readiness 自动触发或后台任务触发。
- preflight 必须要求 readiness=ready，且最近 shadow evidence 不 blocked / incomplete。
- runtime mutation 必须有审计记录和 rollback marker，且能恢复到 switch 前 DNS runtime。
- 仍不声称 fake-ip / fallback-filter / nameserver-policy 已完全 Rust 接管；这些 blocker 存在时必须拒绝 switch。
- UI 必须把 switch 放在危险/实验区域，展示 readiness、shadow evidence、rollback strategy 与不可接管 feature。

不包含：

- 不碰 TUN、transparent proxy、adapter outbound/inbound 或协议栈。
- 不替换 fake-ip cache / fallback-filter / nameserver-policy 的实际 runtime 行为。
- 不做自动 rollout 或默认启用。
- 不执行真实 default DNS runtime mutation；本批次只做 preflight/guard report。

已落地范围：

- 新增 `dns_default_runtime_opt_in_switch_guard` command，默认从当前 runtime DNS config 重新跑 readiness 与 shadow evidence。
- Guard 必须带 `explicitOptIn=true` 才能通过；若 readiness 不是 ready，或 shadow evidence 为 blocked/incomplete，返回 blocked。
- Guard report 输出 rollback plan、activation mode、mutation boundary、blockers/warnings/facts，明确 `mutatesRuntime=false` / `preflightOnly`。
- DNS runtime stats UI 新增实验性 opt-in guard 面板，展示 readiness、shadow evidence、rollback strategy 与 blocker。
- 继续保持只读：不修改 DNS config、不写 active profile、不 reload Mihomo、不切默认 DNS runtime。

### Batch N：Default DNS runtime opt-in executor preflight（本批次，最高风险前置）

Batch M 只证明“是否允许进入 switch 执行器前置”。下一批如果继续推进，仍不应直接默认启用，而是先补齐 executor 的 dry-run preflight 与审计边界：

```text
feat(dns-runtime): add default resolver opt-in executor preflight
```

建议边界：

- executor preflight 必须复用 Batch M guard report，不得绕过 readiness / shadow evidence。
- 必须定义 runtime mutation artifact、audit record、rollback marker 与 superseded state。
- dry-run 只生成候选 runtime DNS mutation diff，不调用 Mihomo reload，不写 active profile。
- 只有 dry-run audit 与 rollback plan 完整后，下一批才可考虑真实 opt-in execution。

不包含：

- 不自动启用 Rust default DNS resolver。
- 不在 blocker 存在时执行任何 mutation。
- 不碰 TUN、transparent proxy、adapter outbound/inbound 或协议栈。
- 不写 active profile、不 reload Mihomo、不执行真实 DNS runtime 切换。

已落地范围：

- 新增 `dns_default_runtime_opt_in_executor_preflight` command，复用 Batch M guard，不允许绕过 readiness / shadow evidence / explicit opt-in。
- Executor preflight report 输出 dry-run mutation diff、audit record、rollback marker、blockers/warnings/facts。
- 明确执行边界：`dryRun=true`、`wouldMutateRuntime=true`、`executed=false`、`reloadMihomo=false`。
- DNS runtime stats UI 新增 executor preflight 面板，展示 guard、diff target、audit event 与 rollback marker。
- 继续保持不修改 DNS config、不写 active profile、不 reload Mihomo、不切默认 DNS runtime。

### Batch O：Default DNS runtime opt-in execution guard（本批次，真实执行前最后门禁）

Batch N 只生成执行器 dry-run 前置报告。若继续推进到真实执行，下一批仍应先做 execution guard，而不是自动启用：

```text
feat(dns-runtime): add default resolver opt-in execution guard
```

建议边界：

- 真实 execution 必须要求 executor preflight ready，且 audit / rollback marker 可持久化。
- 必须先定义 rollback persistence 与 superseded state；执行失败时能回滚到 Mihomo-managed default DNS runtime。
- 必须保持用户显式触发，不随页面打开、readiness、shadow evidence 或后台任务自动执行。
- 真实执行前必须清楚展示 mutation diff、影响范围与不可接管 feature。

不包含：

- 不自动 rollout。
- 不绕过 blocker 执行。
- 不碰 TUN、transparent proxy、adapter outbound/inbound 或协议栈。
- 不写 active profile、不 reload Mihomo、不执行真实 DNS runtime 切换。

已落地范围：

- 新增 `dns_default_runtime_opt_in_execution_guard` command，复用 Batch N executor preflight，不允许绕过 readiness / shadow evidence / guard / executor preflight。
- Execution guard 要求 executor preflight ready，并在执行前持久化 audit record、rollback marker、superseded state。
- Guard report 明确 `executionAllowed`、`userTriggerRequired=true`、`mutatesRuntime=false`、`executed=false`、`reloadMihomo=false`。
- DNS runtime stats UI 新增 execution guard 面板，展示 preflight 状态、persistence、superseded state、audit path、blockers/warnings。
- 继续保持不修改 DNS config、不写 active profile、不 reload Mihomo、不切默认 DNS runtime。

### Batch P：Default DNS runtime limited opt-in execution（已完成 PR #140，真实执行需小范围）

Batch O 已补齐真实执行前的最后门禁与持久化元数据。Batch P 已完成小范围、显式 opt-in、可回滚的 limited execution：

```text
feat(dns-runtime): add limited default resolver opt-in execution
```

建议边界：

- execution 必须读取已持久化的 execution guard metadata，不允许直接从前端临时状态执行。
- 只允许 guard ready、persistence prepared、readiness ready、shadow evidence usable 时执行。
- 执行后必须写入 audit success/failure，失败必须按 rollback marker 恢复 Mihomo-managed default DNS runtime。
- UI 必须明确显示 mutation diff 与 rollback action，并保持危险/实验区域。

不包含：

- 不自动 rollout。
- 不在 blocker / unsupported feature 存在时执行。
- 不碰 TUN、transparent proxy、adapter outbound/inbound 或协议栈。
- 不写 active profile、不 reload Mihomo、不修改 TUN/adapter/protocol runtime。

已落地范围：

- 新增 `dns_default_runtime_limited_opt_in_execution` command，必须先生成并验证已持久化 execution guard metadata。
- Limited execution 只写 Rust-owned default DNS runtime active state 与 execution audit，不写 active profile、不 reload Mihomo。
- 新增 `dns_default_runtime_limited_rollback` command，可把 Rust-owned active state 恢复为 `mihomoManagedDefaultDns` 并写入 rollback audit。
- Report 明确 `metadataVerified`、`rollbackAvailable`、`mutatesRuntime`、`executed`、`reloadMihomo=false`、blockers/warnings/facts。
- DNS runtime stats UI 新增 limited execution 危险区，展示 execution、active state、metadata verification 与 rollback action。

合并结果：

- PR #140 已合并。
- 默认 DNS runtime 已具备 Rust-owned limited execution state 与 rollback audit path。
- 仍没有扩大到 active profile 写入、Mihomo reload、TUN、transparent proxy、adapter outbound/inbound 或协议栈替换。

### Batch Q：Default DNS runtime post-execution observed verification（本批次，执行后验证与 rollback drill）

Batch P 只完成小范围 active-state mutation 与 rollback action。下一批如果继续推进，应先补执行后观测验证与回滚演练，而不是扩大 rollout：

```text
feat(dns-runtime): add post execution observed verification
```

建议边界：

- 只读取 Batch P active state / execution audit / rollback audit。
- 对 active Rust default runtime 做 observed query verification，并和 pre-execution shadow evidence 比较。
- 支持一键 rollback drill report，但不自动 rollback。
- 输出 failure audit，作为是否允许扩大执行范围的依据。

不包含：

- 不自动 rollout。
- 不跳过 rollback drill。
- 不碰 TUN、transparent proxy、adapter outbound/inbound 或协议栈。
- 不自动 rollback。
- 不写 active profile、不 reload Mihomo。

已落地范围：

- 新增 `dns_default_runtime_post_execution_observed_verification` command，读取 Batch P active state、limited execution audit、pre-execution shadow audit 与 rollback marker。
- 对 active Rust default runtime 做 observed query verification，并与 pre-execution shadow status 比较。
- 新增 `dns_default_runtime_rollback_drill` command，只生成 rollback drill report，不执行 rollback。
- Verification report 输出 failure audit、rollback drill readiness、`mutatesRuntime=false`、`reloadMihomo=false` 与 blockers/warnings/facts。
- DNS runtime stats UI 新增 post-execution verification 面板，可显式触发 observed verification 与 rollback drill。
