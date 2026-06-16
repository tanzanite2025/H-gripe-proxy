# ⚡ Clash Verge Optimized

> **重要声明 / Important Notice**
> 
> 📌 这是一个 **个人优化和定制版本** / This is a **personally optimized and customized version**
> 
> - 当前维护仓库 / Maintained repository: [tanzanite2025/clash-verge-optimized](https://github.com/tanzanite2025/clash-verge-optimized)
> - 遵循原项目 **GPLv3 开源协议** / Complies with the original project's **GPLv3 open-source license**
> - 当前主线重点是 **Rust 控制面迁移**、**安全边界收紧** 和 **本仓库可复现打包链** / Current mainline focus: **Rust control-plane migration**, **security boundary hardening**, and **reproducible in-repo packaging**
> - 发布页面 / Releases: [Clash Verge Optimized Releases](https://github.com/tanzanite2025/clash-verge-optimized/releases)

---

## 项目介绍 / Project Introduction

Clash Verge Optimized 是一个基于 [Tauri](https://github.com/tauri-apps/tauri) 构建的个人维护代理桌面应用。

当前仓库的真实职责边界是：

- Rust / Tauri 桌面层负责配置、控制面、运行时协调、诊断、安全边界和平台集成
- `mihomo/` 下的 Go 内核继续承担真实转发、协议栈、TUN、DNS runtime 等尚未迁入 Rust 的能力
- 打包链默认只接受本仓库内受控的 sidecar / service / resources，不再依赖上游 latest 下载链

### 当前主线 / Current Mainline

当前 README 只描述主线仍在维护、且已经落地的能力；更细的迁移批次记录见 [Go → Rust Migration](#go--rust-migration)。

- **桌面控制面**：Tauri / Rust 后端承接配置校验、规则解释、诊断、订阅 artifact、连接/日志事件转发和 app-runtime 编排。
- **Go sidecar 边界**：`mihomo/` 仍是真实转发、协议栈、TUN、DNS runtime 与 adapter/tunnel runtime 的执行方。
- **安全边界**：Release 默认关闭 DevTools；高风险 shell / fs 权限从前端移走；外部 URL、备份恢复、WebDAV TLS 和 CSP 均走显式约束。
- **可复现打包**：构建链优先使用仓库内受控 sidecar / service / resources；修改 `mihomo/` 后必须重编并同步本地 sidecar。

### 已落地能力概览 / Implemented Capabilities

- **配置与规则控制面**：schema 校验、rule parser、rule explain、config diff、diagnostics summary、latency / node selection planner。
- **本地规则数据**：GEOIP、GEOSITE、IP-ASN、SRC-IP-ASN、RULE-SET、PROCESS、UID、DSCP、inbound metadata、logical/sub-rule。
- **订阅与 profile pipeline**：远程 profile → immutable artifact → active marker → runtime 的单一事实链。
- **App runtime 控制面**：应用注册、node pool、DNS/security profile、policy binding、Mihomo projection artifact、session observation/evaluation/leak planning。
- **DNS default runtime 控制面**：readiness、shadow evidence、opt-in execution、post-execution verification、rollback drill、expanded closeout 与 handoff manifest。
- **UI 入口**：高级页提供 app-runtime planning / diagnostics / projection / staged lifecycle / runtime-boundary closeout 面板。

### 下一阶段方向 / Next Direction

- 继续收口 app-runtime staged boundary 后的显式 runtime-apply 决策与审计。
- 继续保持“先 control-plane、再 staged marker、最后显式 runtime apply”的迁移节奏。
- 在 roadmap 明确允许前，不启动 Phase 8，不接管 TUN / protocol runtime / adapter runtime。

---

## 系统要求 / System Requirements

当前桌面层基于 Tauri 2。运行环境以 Tauri / WebView2 / 系统 WebKit 的实际支持矩阵为准；本仓库主线重点验证 Windows x64，Linux/macOS 仍保留跨平台构建目标。

- **Windows**: x64；需要 Microsoft WebView2 Runtime
- **Linux**: x64/arm64；需要系统 WebKitGTK / appindicator 等 Tauri 依赖
- **macOS**: Intel / Apple Silicon；需要 macOS 11+

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
- Go >= 1.21（仅在修改 Mihomo 内核时需要）

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

### Mihomo 内核改动后的打包要求

如果你修改了 `mihomo/` 下的 Go 内核源码，正式打包前需要先重编本地 sidecar。
仓库的 `scripts/prebuild.mjs` 会在打包前校验 `src-tauri/sidecar` 里的 `verge-mihomo` 是否比 `mihomo/` 源码更新。
如果 sidecar 过旧，`pnpm build` 会直接拒绝继续，避免把旧内核打进安装包。

Windows x64 示例：

```powershell
# 1. 重编 Mihomo
Set-Location .\mihomo
$env:CGO_ENABLED='0'
$env:GOARCH='amd64'
$env:GOOS='windows'
$env:GOAMD64='v2'
go build -tags with_gvisor -trimpath -ldflags "-w -s -buildid=" -o bin/mihomo-windows-amd64-v2.exe

# 2. 同步到 Tauri 打包 sidecar
Set-Location ..
Copy-Item .\mihomo\bin\mihomo-windows-amd64-v2.exe `
  .\src-tauri\sidecar\verge-mihomo-x86_64-pc-windows-msvc.exe -Force

# 3. 再执行正式打包
pnpm build
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

迁移路线图和实时进度统一维护在 [`docs/go-to-rust-migration-roadmap.md`](docs/go-to-rust-migration-roadmap.md)。

README 只保留迁移入口、当前状态和边界原则：优先把“不碰真实转发链路”的控制、校验、解释、诊断和调度逻辑迁入 Tauri Rust 后端；Mihomo Go sidecar 在对应 runtime 未迁移前继续负责真实转发、协议栈、TUN、tunnel 和 DNS runtime。

### 当前迁移状态 / Current Status

截至当前主线，Rust 已接管以下 app-facing / control-plane 能力：

- **规则与配置控制面**：配置 schema 校验、规则解析、rule explain、config diff、diagnostics summary、latency planner、node selection planner。
- **本地规则数据**：GEOIP / GEOSITE / IP-ASN / SRC-IP-ASN / RULE-SET / PROCESS / UID / DSCP / inbound metadata / logical rule explain。
- **订阅 artifact pipeline**：远程 profile 更新、不可变 artifact、active artifact marker、legacy profile 写回消除。
- **连接与日志 app-facing 路径**：前端和托盘经 Rust monitor / Tauri event 读取连接、流量、内存和日志事件。
- **DNS default runtime control-plane**：readiness、shadow evidence、opt-in switch guard、executor preflight、limited execution、post-execution verification、rollback drill、expanded stability / hold / reverify / closeout / handoff manifest。
- **App runtime control-plane**：`AppRuntimeStateDocument`、plan / diagnostics、Mihomo projection artifact、session observation/evaluation/leak planning、CRUD/form、demo seed、DNS handoff intake、control-plane completion、staged activation lifecycle、runtime-apply boundary closeout。

### 当前明确边界 / Runtime Boundary

当前 Rust 侧已经形成从 DNS control-plane completion 到 app-runtime staged closeout 的单一事实链：

```text
DNS expanded completion
  -> app-runtime DNS handoff
  -> app-runtime control-plane completion
  -> staged projection marker activation
  -> runtime-apply boundary manifest
```

但这条链路仍然是 **显式控制面 / staged boundary**，不是真实数据面替换：

- 不自动执行 `apply_app_runtime_projection_artifact_to_runtime`
- 不自动 reload Mihomo
- 不自动 rollout / rollback
- 不接管 adapter / tunnel / transparent proxy / protocol runtime
- 不进入 Phase 8（TUN / protocol runtime）

换句话说，Rust 当前已经能完成“计划、诊断、投影、预检、marker、边界 manifest”的闭环；真实转发链路仍由 `mihomo/` Go sidecar 承担，直到 roadmap 明确允许进入下一阶段。

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
├── crates/                     # Rust 库模块
│   ├── tauri-plugin-mihomo/    # Mihomo 内核 Tauri 插件（含 Go 内核绑定）
│   ├── clash-verge-draft/      # 草稿配置管理
│   ├── clash-verge-i18n/       # 国际化支持
│   ├── clash-verge-limiter/    # 流量限速器
│   ├── clash-verge-logging/    # 日志系统
│   ├── clash-verge-signal/     # 信号处理
│   └── clash-verge-xdp/        # XDP 防探测 (Linux)
├── mihomo/                     # Mihomo Go 内核（含规则管理扩展）
├── scripts/                    # 构建 & 工具脚本
├── Cargo.toml                  # Rust workspace 配置
├── package.json                # Node.js 依赖配置
└── README.md                   # 本文件
```

---

## 致谢 / Credits

- **原始项目**: [Clash Verge](https://github.com/zzzgydi/clash-verge) - 早期桌面形态与开源起点
- **当前维护仓库**: [Clash Verge Optimized](https://github.com/tanzanite2025/clash-verge-optimized) - 当前个人维护与持续重构主体
- **框架**: [Tauri](https://github.com/tauri-apps/tauri) - 跨平台应用框架
- **内核**: [Tanzanite Mihomo Optimized Kernel](mihomo/) - 当前仓库内联维护的 Mihomo Go 内核

---

## 支持与反馈 / Support & Feedback

如遇到问题或有任何建议：

- 📝 提交 [Issue](https://github.com/tanzanite2025/clash-verge-optimized/issues)
- 🔄 提交 [Pull Request](https://github.com/tanzanite2025/clash-verge-optimized/pulls)

---

## 许可证 / License

本项目遵循 **GPLv3 License**，详见 [LICENSE](./LICENSE) 文件。

```
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
```

---

**最后更新** / Last Updated: 2026-06-16
**维护者** / Maintainer: tanzanite2025
