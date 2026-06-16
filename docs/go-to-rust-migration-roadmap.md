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
| Phase 7.5 | 应用级代理编排控制面 | 完成（planning / session / readiness UI path） | PR #82/#84-#91/#95-#122；AppRuntimeStateDocument、RuntimePlan、Mihomo projection、diagnostics、session observation/evaluation/leak planning、CRUD/form 管理、聚合诊断动作与 readiness 检查已进入 Rust 单一路径 |

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

当前进度：**完成 planning / session / CRUD / form / observability / readiness UI path（截至 PR #122）**。

目标不是新增一个普通“应用列表”，而是为最终 app-centric proxy orchestration 建立 Rust-owned 数据链：

```text
AppRegistry
  -> AppPolicyBinding
  -> NodePool / DnsProfile / SecurityProfile
  -> RuntimePlan
  -> Mihomo config projection now
  -> Rust runtime execution later
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

前一轮 PR 进度偏慢的主要原因不是技术阻塞，而是切片过细。PR #100-#122 已经把 app-runtime control-plane / diagnostics UI 的基础能力补齐，并完成主面板拆分。后续不能继续停留在零散 UI 增强，应回到 Go → Rust 主线：把 Rust-owned plan 稳定推进到 **可验证、可回滚的 runtime projection artifact**。

### 可以加快做的部分

这些能力都停留在 Rust 控制面、state、diagnostics 或 frontend surface，不直接碰真实流量转发，可合并成较大的功能 PR：

| 优先级 | 可加速方向 | 建议合并方式 | 原因 |
| --- | --- | --- | --- |
| P0 | Runtime projection artifact / diff / validation gate | 下一批应集中做成一个 Rust control-plane PR | 这是从 planning-only 走向 runtime bridge 的主线；先产出可验证 artifact，不直接切 active runtime |
| P0 | App diagnostics 串联 DNS probe / readiness | 已完成聚合诊断动作和 readiness action（PR #111-#112/#121-#122） | 已有 `dns_controlled_runtime_probe`，仍是 opt-in，不切默认 DNS runtime |
| P1 | App runtime CRUD / form 管理面 | 已完成 JSON CRUD、import/export 与常用字段表单（PR #100/#106-#110） | 后续只补必要字段，避免继续做零散 UI |
| P1 | 面板结构维护 | 已完成主面板拆分（PR #113-#120） | 后续功能必须落在已拆分组件中，避免重新形成巨型文件 |
| P2 | Demo / seed import-export | 可在 artifact gate 后再做 | 只有当 artifact / validation chain 稳定后，样例数据才真正服务主线验证 |

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

从提交记录看，Batch D / E 和结构拆分已经完成：Rust-owned app-runtime state 可编辑、可表单化管理、可导入导出、可做绑定 DNS controlled probe、可查看 session 细节、可在 overview matrix 中定位断链，并可一键 readiness。下一步不应继续追加零散 UI，而应把 `RuntimePlan -> Mihomo projection` 变成 Rust 可验证 artifact，为后续 controlled activation 做准备。

### Batch F：Runtime projection artifact / diff / validation gate

建议 PR：

```text
feat(app-runtime): add projection artifact validation gate
```

目标：把当前只显示在 UI 的 `yamlPatch` 升级为 Rust-owned、可审计、可验证、可回滚的 projection artifact，但仍不自动修改 Mihomo runtime。

建议范围：

- 新增 Rust command：对 selected app 或全量 app-runtime state 生成 `AppRuntimeProjectionArtifact`。
- Artifact 至少包含：
  - app id / binding id / referenced node pool / DNS profile / security profile。
  - `RuntimePlan` summary、projection checksum/version、生成时间。
  - YAML patch 或结构化 patch。
  - `mutatesRuntime=false` / `activationMode=staged` 边界字段。
- 复用现有 Rust 校验链路，对 projection 合成后的候选配置做 dry-run validation：
  - config schema validation。
  - rule engine validation / explain。
  - DNS explain / controlled probe summary 引用（probe 仍 opt-in，不自动访问外部 DNS）。
- UI 展示 artifact readiness、diff summary、validation blockers，并允许 export artifact 供 review。

不包含：

- 不切换 active profile。
- 不调用 Mihomo reload / restart。
- 不把 `yamlPatch` 直接写入用户配置作为事实源。
- 不迁默认 DNS resolver runtime。
- 不碰 TUN / transparent proxy / tunnel / adapter outbound/inbound / protocol runtime。

### Batch G：Controlled activation gate（Batch F 之后，小步做）

只有当 Batch F 的 artifact / validation chain 稳定后，才进入 controlled activation：

```text
feat(app-runtime): add opt-in projection activation gate
```

建议边界：

- activation 必须从 Rust artifact 出发，不能从前端临时 YAML 出发。
- activation 必须走现有 profile / artifact / validation 单一路径，并保留 rollback metadata。
- 首版只做显式用户触发，不随页面打开或 readiness 自动执行。
- 首版只桥接到现有 Mihomo runtime；不声称 Rust 已经接管协议栈或强 per-app isolation。

Batch F 是当前最符合大方向的下一步：它把 app-runtime 从“能规划/能诊断”推进到“能生成可验证执行产物”，但仍把高风险 runtime mutation 留到下一批小 PR。
