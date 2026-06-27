# ⚡ Clash Verge Optimized

> **重要声明 / Important Notice**
> 
> 📌 这是一个 **私有维护的 Rust 主导实现** / This is a **privately maintained Rust-led implementation**
> 
> - 当前维护仓库 / Maintained repository: [tanzanite2025/clash-verge-optimized](https://github.com/tanzanite2025/clash-verge-optimized)
> - 授权信息以仓库内 [LICENSE](./LICENSE) 为准 / Licensing follows the repository-local [LICENSE](./LICENSE)
> - 当前主线重点是 **Rust 控制面迁移**、**安全边界收紧** 和 **本仓库可复现打包链** / Current mainline focus: **Rust control-plane migration**, **security boundary hardening**, and **reproducible in-repo packaging**
> - 发布页面 / Releases: [Clash Verge Optimized Releases](https://github.com/tanzanite2025/clash-verge-optimized/releases)

---

## 项目介绍 / Project Introduction

Clash Verge Optimized 是一个基于 [Tauri](https://github.com/tauri-apps/tauri) 与 Rust 主导架构构建的私有维护代理桌面应用。

当前仓库的真实职责边界是：

- Rust / Tauri 桌面层负责配置、控制面、运行时协调、诊断、安全边界和平台集成
- 数据面（真实转发、协议栈、路由、DNS、TUN、conntrack、TLS/REALITY、传输）由**进程内纯 Rust 内核 `learn-gripe`**（crate `learn-gripe`）承载；不再打包、启动或依赖 Go/Mihomo sidecar 二进制
- 打包链默认只接受本仓库内受控的 service / resources / geodata，不依赖外部 latest 下载链

### 当前主线 / Current Mainline

当前 README 只描述主线仍在维护、且已经落地的能力；更细的迁移批次记录见 [Go → Rust Migration](#go--rust-migration)。

- **桌面控制面**：Tauri / Rust 后端承接配置校验、规则解释、诊断、订阅 artifact、连接/日志事件转发和 app-runtime 编排。
- **进程内 Rust 内核**：`learn-gripe` 在 `start_core()` 内启动，单端口 mixed inbound（SOCKS5 + HTTP）按首字节分流，按连接规则路由（`rule` 模式跑完整 router，`global`/单节点模式解析当前 select 组），承载协议转发、UDP、DNS 与 TUN 数据面。
- **安全边界**：Release 默认关闭 DevTools；高风险 shell / fs 权限从前端移走；外部 URL、备份恢复、WebDAV TLS 和 CSP 均走显式约束。
- **可复现打包**：构建链优先使用仓库内受控 service / resources / geodata；内核随应用一起编译，无需外部二进制。

### 已落地能力概览 / Implemented Capabilities

- **配置与规则控制面**：schema 校验、rule parser、rule explain、config diff、diagnostics summary、latency / node selection planner。
- **本地规则数据**：GEOIP、GEOSITE、IP-ASN、SRC-IP-ASN、RULE-SET、PROCESS、UID、DSCP、inbound metadata、logical/sub-rule。
- **订阅与 profile pipeline**：远程 profile → immutable artifact → active marker → runtime 的单一事实链。
- **App runtime 控制面**：应用注册、node pool、DNS/security profile、policy binding、Mihomo projection artifact、session observation/evaluation/leak planning。
- **DNS default runtime 控制面**：readiness、shadow evidence、opt-in execution、post-execution verification、rollback drill、expanded closeout 与 handoff manifest。
- **UI 入口**：高级页提供 app-runtime planning / diagnostics / projection / staged lifecycle / runtime-boundary closeout 面板。

### Next Direction

- Go/Mihomo sidecar 已完全退役；后续重点是把仍为 read-only/bounded 的运行时证据，在 operator 批准后推进到默认路径 cutover（DNS / route / TUN / 转发）。
- Rust evidence 默认保持 bounded/read-only，除非 operator 批准的 cutover 显式允许特权 DNS、route、TUN、plugin 或转发变更。
- 真机 TUN 全局默认路由捕获仍待在真实硬件上验证（见 roadmap Phase 4）。

---

## 系统要求 / System Requirements

当前桌面层基于 Tauri 2。本仓库已收敛为 Windows-only：后端平台代码、Cargo 依赖与 CI 均只保留 Windows，不再提供 Linux/macOS 构建目标。

- **Windows**: x64；需要 Microsoft WebView2 Runtime

---

## 安装 / Installation

### 当前发布版 / Current Release
请访问 [Clash Verge Optimized Releases](https://github.com/tanzanite2025/clash-verge-optimized/releases) 获取当前维护版本。

### 项目仓库 / Repository
请访问 [tanzanite2025/clash-verge-optimized](https://github.com/tanzanite2025/clash-verge-optimized) 查看源码与说明。

---

## 快速开始 / Quick Start

### 环境要求

- Node.js >= 18.0
- pnpm 10.33.0（见 `packageManager`）
- Rust 1.95.0（见 `rust-toolchain.toml`）
- Tauri CLI（通过项目脚本调用即可）

### 编译构建

```bash
# 安装前端依赖
pnpm install

# 开发模式运行
pnpm dev

# 仅前端构建检查
pnpm web:build

# 生成正式安装包
pnpm build

# 更快的测试打包（使用 fast-release）
pnpm build:fast
```

正式安装包默认输出到：

```text
target/release/bundle/nsis/
```

### 本地 IP 元数据数据库

为了让顶部出口信息、共享缓存、时区伪装和诊断页统一使用单一本地事实源，当前版本约定本地 MMDB 文件按下面方式接入：

- `GeoLite2-City.mmdb` 或 `City.mmdb`
  作用：提供 `country / region / city / timezone`
- `GeoLite2-ASN.mmdb` 或 `ASN.mmdb`
  作用：提供 `ASN / organization`
- `Country.mmdb`、`country.mmdb` 或 `GeoLite2-Country.mmdb`
  作用：在没有 City 库时补充国家信息

放置规则：

- 开发和打包时放入 `src-tauri/resources/`
- 已安装版本可直接放入应用数据目录
  Windows: `%APPDATA%/io.github.tanzanite2025.clash-verge-optimized/`

程序启动时会把 `resources/` 中较新的数据库复制到应用数据目录，所以正式打包时只要把这些文件带进 `src-tauri/resources/`，运行态就会自动接管。

注意：

- 只有 `City.mmdb` 链路到位后，timezone 才能来自本地精确库
- 如果只有 `Country.mmdb`，程序仍可运行，但 timezone 只能退回国家级推断，不属于精确城市级结果

---

## Go → Rust Migration

**迁移已完成**：应用不再打包、启动或依赖 Go/Mihomo sidecar 二进制。数据面在进程内由纯 Rust 内核 `learn-gripe` 承载，原 Mihomo 兼容 DTO 已迁入 `crates/clash-dtos`。按 Phase 记录的详细实现台账见 [`docs/go-to-rust-migration-roadmap.md`](docs/go-to-rust-migration-roadmap.md)。

### 现状 / Current Status

- **Mihomo sidecar 已完全退役**：树内无二进制，打包 / release 链（`prebuild.mjs`、`tauri.conf.json`、`tauri.linux.conf.json` 的 `externalBin`、`release.yml`）不再引用；`tauri-plugin-mihomo` crate 已删除，控制器 IPC 客户端已移除，所有原 IPC 命令改为进程内执行。
- **数据面 = `learn-gripe`（进程内）**：单端口 mixed inbound（SOCKS5 + HTTP）、按连接规则路由、协议转发、UDP、DNS、TUN 全在 Rust 内；不再有任何 Mihomo 所属运行面。
- **控制面成熟**：校验、planning、projection artifact、订阅 pipeline、监控、审计、前端类型面均 Rust 所有。

### 仍受 gate 的部分 / Still Gated

特权默认路径 cutover（生产 DNS apply/restore、route/TUN 变更、转发 cutover）默认仍是 bounded / read-only evidence，需 operator 显式批准才执行；真机 TUN 全局默认路由捕获仍待真机验证。详见 roadmap 的 Phase 4 / Phase 5。

---

## 项目结构 / Project Structure

```
clash-verge-optimized/
├── src/                        # 前端源代码 (TypeScript/React/TailwindCSS)
│   ├── components/
│   │   ├── advanced/           # 高级配置面板（住宅代理池、安全策略等）
│   │   ├── home/               # 首页卡片组件
│   │   ├── proxy/              # 代理页面组件（链式代理抽屉、代理组等）
│   │   ├── security/           # 安全相关组件
│   │   ├── setting/            # 设置页面组件
│   │   └── tailwind/           # TailwindCSS 基础组件库（Dialog, Button, Paper 等）
│   ├── hooks/                  # 自定义 React Hooks
│   ├── locales/                # i18n 多语言文件
│   ├── pages/                  # 页面组件
│   ├── providers/              # React Context Providers
│   ├── services/               # Tauri 命令封装与 API 服务
│   └── utils/                  # 工具函数（URI 解析器等）
├── src-tauri/                  # Tauri 后端源代码 (Rust)
│   ├── src/
│   │   ├── cmd/                # Tauri 命令处理器
│   │   ├── config/             # 配置管理（AdvancedConfig、RuntimeConfig）
│   │   ├── core/               # 核心逻辑（CoreManager、DNS runtime、app-runtime、rule engine）
│   │   ├── enhance/            # 配置增强（StableEgress、ResidentialChain）
│   │   └── feat/               # 业务功能模块
│   └── capabilities/           # Tauri 权限配置
├── crates/                     # Rust 库模块 / workspace 成员
│   ├── learn-gripe/            # 进程内纯 Rust 代理内核（数据面）
│   ├── clash-dtos/             # Mihomo 兼容 DTO + ts-rs TypeScript bindings
│   ├── clash-verge-draft/      # 草稿配置管理
│   ├── clash-verge-i18n/       # 国际化支持
│   ├── clash-verge-limiter/    # 流量限速器
│   ├── clash-verge-logging/    # 日志系统（统一日志链路）
│   ├── clash-verge-signal/     # 信号处理
│   ├── clash-verge-service-ipc/# 特权 helper service IPC
│   ├── sysproxy/               # 系统代理设置
│   └── tauri-plugin-clash-verge-sysinfo/  # 系统信息 Tauri 插件
├── scripts/                    # 构建 & 工具脚本
├── Cargo.toml                  # Rust workspace 配置
├── package.json                # Node.js 依赖配置
└── README.md                   # 本文件
```

---

## 项目归属 / Ownership

本仓库是 tanzanite2025 私有维护的 Rust 主导实现。README 不再保留外部致谢式定位；当前说明只描述本仓库自己的架构、边界、构建链和迁移路线。

---

## 支持与反馈 / Support & Feedback

如遇到问题或有任何建议：

- 📝 提交 [Issue](https://github.com/tanzanite2025/clash-verge-optimized/issues)
- 🔄 提交 [Pull Request](https://github.com/tanzanite2025/clash-verge-optimized/pulls)

---

## 许可证 / License

授权信息以仓库内 [LICENSE](./LICENSE) 文件为准。

---

**最后更新** / Last Updated: 2026-06-26
**维护者** / Maintainer: tanzanite2025
