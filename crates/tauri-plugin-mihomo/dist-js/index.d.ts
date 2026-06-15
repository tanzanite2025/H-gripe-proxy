import { BaseConfig, BufferPoolStats, ConnTrafficSnapshot, Connections, CoreUpdaterChannel, DnsMetrics, EgressStatus, EngineStats, Groups, HotReloadStatus, LogLevel, MihomoVersion, PerfStats, Proxies, Proxy, ProxyDelay, ProxyProvider, ProxyProviders, Rule, RuleProviders, RuleTrafficSnapshot, Rules, TLSFingerprintStats, TLSRotationResult, XDPStatus } from "./bindings";
export * from "./bindings";
export type MihomoGroupDelay = Record<string, number>;
/**
 * 更新控制器地址
 * @param controller 控制器地址, 例如：127.0.0.1:9090
 */
export declare function updateController(controller: string): Promise<void>;
/**
 * 更新控制器的密钥
 * @param secret 控制器的密钥
 */
export declare function updateSecret(secret: string): Promise<void>;
/**
 * 获取 Mihomo 版本信息
 */
export declare function getVersion(): Promise<MihomoVersion>;
/**
 * 清除 FakeIP 缓存
 */
export declare function flushFakeIp(): Promise<void>;
/**
 * 清除 DNS 缓存
 */
export declare function flushDNS(): Promise<void>;
/**
 * 获取 DNS 性能指标（缓存命中率、查询延迟、服务器状态）
 */
export declare function getDnsMetrics(): Promise<DnsMetrics>;
/**
 * DNS 预解析（预热常用域名缓存）
 */
export declare function dnsWarmup(): Promise<void>;
/**
 * 获取所有连接信息
 * @returns 所有连接信息
 */
export declare function getConnections(): Promise<Connections>;
/**
 * 关闭所有连接
 */
export declare function closeAllConnections(): Promise<void>;
/**
 * 关闭指定连接
 * @param connectionId 连接 ID
 */
export declare function closeConnection(connectionId: string): Promise<void>;
/**
 * 获取所有代理组信息
 * @returns 所有代理组信息
 */
export declare function getGroups(): Promise<Groups>;
/**
 * 获取指定代理组信息
 * @param groupName 代理组名称
 * @returns 指定代理组信息
 */
export declare function getGroupByName(groupName: string): Promise<Proxy>;
/**
 * 对指定代理组进行延迟测试
 *
 * 注：返回值中不包含超时的节点
 * @param groupName 代理组名称
 * @param testUrl 测试 url
 * @param timeout 超时时间（毫秒）
 * @param keepFixed 是否保留已固定的节点, 默认 false
 * @returns 代理组中代理节点的延迟，返回数据中无超时节点的数据
 */
export declare function delayGroup(groupName: string, testUrl: string, timeout: number, keepFixed?: boolean): Promise<MihomoGroupDelay>;
/**
 * 获取所有代理提供者信息
 * @returns 所有代理提供者信息
 */
export declare function getProxyProviders(): Promise<ProxyProviders>;
/**
 * 获取指定的代理提供者信息
 * @param providerName 代理提供者名称
 * @returns 代理提供者信息
 */
export declare function getProxyProviderByName(providerName: string): Promise<ProxyProvider>;
/**
 * 更新代理提供者信息
 * @param providerName 代理提供者名称
 */
export declare function updateProxyProvider(providerName: string): Promise<void>;
/**
 * 对指定的代理提供者进行健康检查
 * @param providerName 代理提供者名称
 */
export declare function healthcheckProxyProvider(providerName: string): Promise<void>;
/**
 * 对指定代理提供者下的指定节点（非代理组）进行健康检查, 并返回新的延迟信息
 * @param providerName 代理提供者名称
 * @param proxyName 代理节点名称 (非代理组)
 * @param testUrl 测试 url
 * @param timeout 超时时间
 * @returns 该代理节点的延迟
 */
export declare function healthcheckNodeInProvider(providerName: string, proxyName: string, testUrl: string, timeout: number): Promise<ProxyDelay>;
/**
 * 获取所有代理信息
 * @returns 所有代理信息
 */
export declare function getProxies(): Promise<Proxies>;
/**
 * 获取指定代理信息
 * @param proxyName 代理名称
 * @returns 代理信息
 */
export declare function getProxyByName(proxyName: string): Promise<Proxy | null>;
/**
 * 为指定代理选择节点
 *
 * 一般为指定代理组下使用指定的代理节点 【代理组/节点】
 * @param groupName 代理组名称
 * @param node 代理节点
 */
export declare function selectNodeForGroup(groupName: string, node: string): Promise<void>;
/**
 * 指定代理组下不再使用固定的代理节点
 *
 * 一般用于自动选择的代理组（例如：URLTest 类型的代理组）下的节点
 * @param groupName 代理组名称
 */
export declare function unfixedProxy(groupName: string): Promise<void>;
/**
 * 对指定代理进行延迟测试
 *
 * 一般用于代理节点的延迟测试，也可传代理组名称（只会测试代理组下选中的代理节点）
 * @param proxyName 代理节点名称
 * @param testUrl 测试 url
 * @param timeout 超时时间
 * @returns 该代理节点的延迟信息
 */
export declare function delayProxyByName(proxyName: string, testUrl: string, timeout: number): Promise<ProxyDelay>;
/**
 * 获取所有规则信息
 * @returns 所有规则信息
 */
export declare function getRules(): Promise<Rules>;
/**
 * 禁用或启用规则
 * @param payload 规则索引到禁用状态的映射
 */
export declare function disableRules(payload: Record<number, boolean>): Promise<void>;
/**
 * Soft-delete a rule by index
 * @param index Rule index
 */
export declare function deleteRule(index: number): Promise<void>;
/**
 * Create a new runtime rule
 * @param ruleType Rule type (e.g. DOMAIN, IP-CIDR, AND, OR, NOT)
 * @param payload Rule payload
 * @param proxy Target proxy/group
 * @param source Optional source tag (e.g. "security:policy-name"). Defaults to "runtime" if not specified.
 * @param subRule Optional sub-rule list name. If set, rule is inserted into that sub-rule list instead of global rules.
 * @param position Optional insertion position: "prepend" or "append" (default).
 * @returns Index of the created rule
 */
export declare function createRule(ruleType: string, payload: string, proxy: string, source?: string, subRule?: string, position?: string): Promise<number>;
/**
 * Get all sub-rules
 * @returns Map of sub-rule name to rule arrays
 */
export declare function getSubRules(): Promise<Record<string, Rule[]>>;
/**
 * Delete sub-rules by source prefix
 * @param name Sub-rule list name
 * @param sourcePrefix Optional source prefix filter. Defaults to "security:" if not specified.
 * @returns Number of deleted rules
 */
export declare function deleteSubRuleBySource(name: string, sourcePrefix?: string): Promise<number>;
/**
 * 获取所有规则提供者信息
 * @returns 所有规则提供者信息
 */
export declare function getRuleProviders(): Promise<RuleProviders>;
/**
 * 更新规则提供者信息
 * @param providerName 规则提供者名称
 */
export declare function updateRuleProvider(providerName: string): Promise<void>;
/**
 * 获取基础配置
 * @returns 基础配置
 */
export declare function getBaseConfig(): Promise<BaseConfig>;
/**
 * 重新加载配置
 * @param force 强制更新
 * @param configPath 配置文件路径
 */
export declare function reloadConfig(force: boolean, configPath: string): Promise<void>;
/**
 * 更改基础配置
 * @param data 基础配置更改后的内容, 例如：{"tun": {"enabled": true}}
 */
export declare function patchBaseConfig(data: Record<string, any>): Promise<void>;
/**
 * 更新 Geo
 */
export declare function updateGeo(): Promise<void>;
/**
 * 重启核心
 */
export declare function restart(): Promise<void>;
/**
 * 升级核心，将当前运行中的核心升级到选择的通道的最新版
 * @param channel 升级通道, 默认 auto
 *    - release: 稳定版
 *    - alpha: 测试版
 *    - auto: 根据当前运行的核心版本自动选择升级通道
 * @param force 是否强制升级，默认 false
 *    - false: 若当前版本为最新版，返回当前为最新版的错误，不再执行升级操作, 否则下载最新版，覆盖升级
 *    - true: 直接下载最新版，强制覆盖升级
 */
export declare function upgradeCore(channel?: CoreUpdaterChannel, force?: boolean): Promise<void>;
/**
 * 更新 UI
 */
export declare function upgradeUi(): Promise<void>;
/**
 * 更新 Geo
 */
export declare function upgradeGeo(): Promise<void>;
/**
 * 获取引擎统计（活跃连接数、追踪连接数）
 */
export declare function getEngineStats(): Promise<EngineStats>;
/**
 * 获取 Top N 带宽连接
 */
export declare function getTopConnections(): Promise<ConnTrafficSnapshot[]>;
/**
 * 获取缓冲池统计
 */
export declare function getBufferPoolStats(): Promise<BufferPoolStats>;
/**
 * 获取规则流量统计
 */
export declare function getRuleTraffic(): Promise<Record<string, RuleTrafficSnapshot>>;
/**
 * 获取出口状态
 */
export declare function getEgressStatus(): Promise<EgressStatus>;
/**
 * 获取 TLS 指纹统计
 */
export declare function getTlsFingerprintStats(): Promise<TLSFingerprintStats>;
/**
 * 强制 TLS 指纹轮换
 */
export declare function forceTlsRotation(): Promise<TLSRotationResult>;
/**
 * 获取性能统计
 */
export declare function getPerfStats(): Promise<PerfStats>;
/**
 * 获取热重载状态
 */
export declare function getHotReloadStatus(): Promise<HotReloadStatus>;
/**
 * 获取 XDP 状态
 */
export declare function getXdpStatus(): Promise<XDPStatus>;
