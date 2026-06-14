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
| Phase 4C | 进程 / UID / DSCP / inbound 元数据规则 | 部分完成 | PR #17-#25；已完成 exact/regex process、UID、DSCP、`IN-TYPE` / `IN-USER` / `IN-NAME` |

## 下一阶段推荐顺序

### A. `IP-ASN` / `SRC-IP-ASN` 本地匹配

**状态：已完成（PR #15）。**

**优先级：最高。复杂度：低。**

这是下一步最简单的实现，因为当前代码已经具备三块基础：

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

**优先级：第二。复杂度：中。**

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
11. `AND` / `OR` / `NOT` / `SUB-RULE`：已完成（本 PR）。

说明：

- ASN 与 RULE-SET 仍属于“数据查表 + 规则复用”，风险低。
- PROCESS/UID/IN-* 开始涉及 OS、进程权限、入口监听器上下文，复杂度会明显上升。
- 当前 Rust 侧只消费 `ConnectionMeta` 已提供的 process / uid / dscp / inbound metadata；不负责 OS 级进程发现或 inbound runtime 采集。
- Phase 4 外部数据类规则已闭环；后续建议进入 Phase 5 的规则预览 / 配置 explain 等控制器外围逻辑。

#### Phase 5：控制器外围逻辑 Rust 化

当前进度：

1. 规则预览 / 规则解释器：已完成（PR #31）。
2. 配置 diff / explain：已完成（PR #33）。
3. runtime diagnostics 聚合：已完成（PR #34）。
4. latency test 调度层：已完成（PR #35）。
5. 节点选择策略的外层编排：下一批候选。

这类逻辑不碰真实转发链路，适合继续迁。

Phase 5 的删除边界：

- Rust 已接管的控制器外围能力，不再保留前端或 Go 侧同类预览 / explain / 规划兜底入口。
- Go `mihomo/` 中的 rule matching、`URLTest`、provider health check、tunnel scheduler 仍属于真实 runtime / forwarding 数据来源；在 Rust 尚未接管 runtime 前不能删除。
- 每次迁完一个外围能力，应同步删除旧 wrapper / fallback，并在测试里固定调用 Rust Tauri command。

#### Phase 6：DNS 解析迁移

候选技术：

- `hickory-resolver`
- `hickory-proto`

建议拆成三步：

1. Rust DNS 配置校验与 explain。
2. Rust DNS probe / health check。
3. Rust DNS resolver runtime。

不要一上来替换 Go 的 DNS runtime，否则会同时碰缓存、fake-ip、fallback-filter、nameserver-policy。

#### Phase 7：连接统计 / 流量监控

这块看起来简单，但要确认数据来源：

- 如果只是 UI 展示，可以先做 Rust 聚合缓存。
- 如果需要真实 per-connection 统计，仍依赖 Go tunnel/runtime 事件。

建议先做“Rust 侧统一指标模型”，不要先拆 tunnel。

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

按当前状态，下一张实现 PR 建议进入：

```text
feat: add Rust rule preview / explain support
```

范围只包含：

- 复用已迁移的 Rust rule engine。
- 为规则预览 / explain 输出统一结构。
- 继续保持 Go sidecar 只负责 runtime 转发链路。
- focused tests：命中规则解释、未命中 fallthrough、RULE-SET / SUB-RULE 展示。

不包含：

- DNS runtime。
- 协议栈 / TUN / tunnel。
- Go sidecar 替换。
