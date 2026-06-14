# ⚡ Clash Verge Optimized

> **重要声明 / Important Notice**
> 
> 📌 这是一个 **个人优化和定制版本** / This is a **personally optimized and customized version**
> 
> - 当前维护仓库 / Maintained repository: [tanzanite2025/clash-verge-optimized](https://github.com/tanzanite2025/clash-verge-optimized)
> - 遵循原项目 **GPLv3 开源协议** / Complies with the original project's **GPLv3 open-source license**
> - 本版本针对 **UI优化**、**安全加固** 和 **功能增强** 进行了深度改进 / This version includes deep improvements for **UI optimization**, **security hardening**, and **feature enhancements**
> - 发布页面 / Releases: [Clash Verge Optimized Releases](https://github.com/tanzanite2025/clash-verge-optimized/releases)

---

## 项目介绍 / Project Introduction

Clash Verge Optimized 是一个基于 [Tauri](https://github.com/tauri-apps/tauri) 框架的 Mihomo (Clash Meta) 图形用户界面应用。

本仓库是 **个人深度优化版本**，在以下方面进行了全面改进：

### 🎨 UI 优化
- **一体化卡片式首页** — 系统信息、当前代理、代理模式、TUN 开关等整合为卡片布局
- **无边框设计** — 卡片去除边框虚线，视觉更简洁统一
- **底部抽屉式代理链** — 链式代理配置以全宽底部抽屉呈现，4列网格排列节点，支持拖拽排序
- **住宅代理池集成** — 抽屉内直接配置住宅代理出口，无需切换页面
- **顶部 TAB 栏优化** — 增大拖拽区域，缩小 TAB 间距
- **弹窗居中补偿** — 对话框在 TAB 栏以下区域居中显示

### 🔒 安全加固
- **DevTools 构建门控** — 开发者工具仅在 Debug 构建中可用，Release 自动禁用
- **Tauri 权限最小化** — 移除 shell:allow-execute/kill/spawn、fs:allow-write-file 等高危权限
- **CSP 白名单收紧** — connect-src 仅允许本地控制平面，远程域名显式白名单
- **WebDAV TLS 验证** — 不再默认接受无效证书
- **ZIP 路径穿越防护** — 备份恢复时校验解压路径
- **外部 URL 校验** — 仅允许 http/https 协议且含 host 的 URL 打开
- **前端 plugin-fs/plugin-shell 移除** — 文件操作和进程管理全部走后端命令

### 🚀 功能增强
- **链式代理 (Proxy Chain)** — 通过 dialer-proxy 构建多跳链式代理，支持拖拽排序、连接/断开
- **住宅代理池 (Residential Pool)** — 支持 SOCKS5/HTTP/SS/VMess/Trojan 类型住宅代理，可配置为链式出口
- **稳定出口策略 (Stable Egress)** — 基于 egress_identity 的出口身份管理，自动构建 VPS→住宅链式路由
- **运行时规则管理** — 支持规则启用/禁用/软删除/创建，来源标记（profile/provider/runtime）
- **URI 解析器** — 支持 ss/ssr/vmess/vless/trojan/trojan-go/anytls/hysteria/hysteria2/tuic/wireguard 等协议
- **多路径 (Multipath)** — 多路径并发传输配置
- **流量填充 (Traffic Padding)** — 流量混淆与填充对抗检测
- **XDP 防探测 (Anti-Probe)** — Linux XDP 层面的主动探测防御
- **IP 信誉度 (IP Reputation)** — 出口 IP 信誉度监控
- **黑洞断路器 (Blackhole Breaker)** — 检测并绕过黑洞路由
- **时区伪装 (Timezone Spoof)** — 出口时区一致性伪装

### 🐛 Bug 修复
- 修复链式代理切换页面自动弹出的问题
- 修复代理组节点在链式模式下折叠的问题
- 修复 Tab 栏拖拽区域不足的问题
- 修复弹窗不在可视区域居中的问题
- 统一 CmdResult 类型，消除 Result<T, String> 与 Result<T, SmartString> 混用
- 移除 CoordinatorConfig 配置双源问题，统一为 AdvancedConfig 单一配置源

### 许可证声明

本项目严格遵循原始项目的 **GPLv3 License**，符合开源协议要求。所有修改都以开源的方式进行。

---

## 系统要求 / System Requirements

- **Windows**: Windows 7 SP1 及以上版本 (x64/x86)
- **Linux**: Ubuntu 20.04 及以上版本 (x64/arm64)
- **macOS**: 11.0 及以上版本 (Intel/Apple Silicon)

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
- pnpm
- Rust >= 1.70
- Tauri CLI
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
仓库的 `scripts/prebuild.mjs` 会在打包前校验：
`src-tauri/sidecar` 里的 `verge-mihomo` 是否比 `mihomo/` 源码更新。
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

## GO 内核转 Rust 迁移进度 / Go-to-Rust Migration

迁移路线图详见 [`docs/go-to-rust-migration-roadmap.md`](docs/go-to-rust-migration-roadmap.md)。
当前策略是：优先把“不碰真实转发链路”的控制与校验逻辑迁入 Tauri Rust 后端，Mihomo Go sidecar 继续负责 runtime 转发、协议栈、TUN 和 DNS runtime。

### 当前状态

Phase 4「规则引擎外部数据类型」已闭环，以下规则能力已进入 Rust 本地规则引擎路径：

- `IP-ASN` / `SRC-IP-ASN`
- `RULE-SET`
- `PROCESS-NAME` / `PROCESS-PATH`
- `PROCESS-NAME-REGEX` / `PROCESS-PATH-REGEX`
- `UID`
- `DSCP`
- `IN-TYPE` / `IN-USER` / `IN-NAME`
- `PROCESS-NAME-WILDCARD` / `PROCESS-PATH-WILDCARD`
- `AND` / `OR` / `NOT` / `SUB-RULE`

Rust 侧规则匹配继续遵循 fail-soft 原则：如果 `ConnectionMeta` 或本地 provider / sub-rule 数据缺失，应当继续 fallthrough，而不是 panic 或回退到 Go sidecar 做同类预览校验。

### 下一阶段

下一步建议进入 Phase 5「控制器外围逻辑 Rust 化」：

- 规则预览 / 规则解释器
- 配置 diff / explain
- runtime diagnostics 聚合
- latency test 调度层
- 节点选择策略的外层编排

近期不建议直接替换 Go sidecar，也不建议先迁 DNS runtime、协议栈、TUN、tunnel 或 adapter 转发链路。

---

## 项目结构 / Project Structure

```
clashverge-clean/
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
│   │   ├── core/               # 核心逻辑（CoreManager、Backup、Coordinator）
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

- **原始项目**: [Clash Verge](https://github.com/zzzgydi/clash-verge) - 初始创意和架构
- **当前维护仓库**: [Clash Verge Optimized](https://github.com/tanzanite2025/clash-verge-optimized) - 当前版本维护与定制优化
- **框架**: [Tauri](https://github.com/tauri-apps/tauri) - 跨平台应用框架
- **内核**: [Tanzanite Mihomo Optimized Kernel](mihomo/) - 本项目维护的 MIHOMO 代理核心

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

**最后更新** / Last Updated: 2026-06-14  
**维护者** / Maintainer: tanzanite2025
