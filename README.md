# ⚡ Clash Verge Optimized

> **重要声明 / Important Notice**
> 
> 📌 这是一个 **个人优化和定制版本** / This is a **personally optimized and customized version**
> 
> - 当前维护仓库 / Maintained repository: [tanzanite2025/clash-verge-optimized](https://github.com/tanzanite2025/clash-verge-optimized)
> - 遵循原项目 **GPLv3 开源协议** / Complies with the original project's **GPLv3 open-source license**
> - 本版本针对 **UI优化** 和 **BUG修复** 进行了个性化改进 / This version includes personalized improvements for **UI optimization** and **bug fixes**
> - 发布页面 / Releases: [Clash Verge Optimized Releases](https://github.com/tanzanite2025/clash-verge-optimized/releases)

---

## 项目介绍 / Project Introduction

Clash Verge Optimized 是一个基于 [Tauri](https://github.com/tauri-apps/tauri) 框架的 Clash Meta 图形用户界面应用。

本仓库是 **个人优化版本**，在以下方面进行了改进：

- 🎨 **UI 优化** - 改进的用户界面设计和交互体验
- 🐛 **Bug 修复** - 修复已知问题并提升稳定性
- 🔧 **功能优化** - 根据个人使用习惯进行定制和优化
- 🚀 **性能改进** - 更流畅的应用运行体验

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

### 编译构建

`ash
# 安装依赖
pnpm install

# 开发模式运行
cargo make dev

# 生成可执行文件
cargo make build
`

---

## 项目结构 / Project Structure

`
clashverge/
├── src/                    # 前端源代码 (TypeScript/React)
├── src-tauri/             # Tauri 后端源代码 (Rust)
├── crates/                # Rust 库模块
├── scripts/               # 构建脚本
├── docs/                  # 文档和预览图
├── Cargo.toml            # Rust 依赖配置
├── package.json          # Node.js 依赖配置
└── README.md             # 本文件
`

---

## 致谢 / Credits

- **原始项目**: [Clash Verge](https://github.com/zzzgydi/clash-verge) - 初始创意和架构
- **当前维护仓库**: [Clash Verge Optimized](https://github.com/tanzanite2025/clash-verge-optimized) - 当前版本维护与定制优化
- **框架**: [Tauri](https://github.com/tauri-apps/tauri) - 跨平台应用框架
- **内核**: [Mihomo](https://github.com/MetaCubeX/mihomo) - Clash 代理核心

---

## 支持与反馈 / Support & Feedback

如遇到问题或有任何建议：

- 📝 提交 [Issue](https://github.com/tanzanite2025/clash-verge-optimized/issues)
- 🔄 提交 [Pull Request](https://github.com/tanzanite2025/clash-verge-optimized/pulls)

---

## 许可证 / License

本项目遵循 **GPLv3 License**，详见 [LICENSE](./LICENSE) 文件。

`
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
`

---

**最后更新** / Last Updated: 2026-05-26  
**维护者** / Maintainer: tanzanite2025
