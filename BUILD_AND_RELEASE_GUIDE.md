# 📦 打包和发布指南

## 📋 目录
- [版本号管理](#版本号管理)
- [本地打包](#本地打包)
- [CI/CD 自动打包](#cicd-自动打包)
- [版本号同步](#版本号同步)
- [打包流程详解](#打包流程详解)
- [常见问题](#常见问题)

---

## 🔢 版本号管理

### 当前版本
项目当前版本：**0.0.3**

### 版本号位置
版本号需要在以下 **4 个文件** 中保持同步：

1. **package.json**
   ```json
   {
     "version": "0.0.3"
   }
   ```

2. **Cargo.toml** (workspace root)
   ```toml
   # 此文件不包含版本号，版本号在 src-tauri/Cargo.toml 中
   ```

3. **src-tauri/Cargo.toml**
   ```toml
   [package]
   version = "0.0.3"
   ```

4. **src-tauri/tauri.conf.json**
   ```json
   {
     "version": "0.0.3"
   }
   ```

### 自动更新版本号

使用 `release-version.mjs` 脚本自动更新所有文件中的版本号：

```bash
# 更新到指定版本
pnpm release-version 1.2.3
pnpm release-version v1.2.3

# 使用预发布标签
pnpm release-version 1.2.3-beta
pnpm release-version beta        # 基于当前版本添加 -beta
pnpm release-version alpha       # 基于当前版本添加 -alpha
pnpm release-version rc          # 基于当前版本添加 -rc

# 自动构建版本（带时间戳和 commit hash）
pnpm release-version autobuild           # 格式: 0.0.3+autobuild.1004.cc39b27
pnpm release-version autobuild-latest    # 使用最新 Tauri 相关 commit
pnpm release-version deploytest          # 格式: 0.0.3+deploytest.1004.cc39b27
```

### 版本号格式

支持的版本号格式（遵循 Semver）：

- **正式版本**: `1.2.3`, `v1.2.3`
- **预发布版本**: `1.2.3-alpha`, `1.2.3-beta`, `1.2.3-rc`
- **构建元数据**: `1.2.3+autobuild.1004.cc39b27`
- **组合**: `1.2.3-beta+build.123`

---

## 🏗️ 本地打包

### 前置准备

1. **安装依赖**
   ```bash
   pnpm install
   ```

2. **准备资源文件**（自动下载 mihomo core、服务文件、GeoIP 数据等）
   ```bash
   pnpm prebuild
   
   # 强制重新下载所有资源
   pnpm prebuild --force
   ```

### 打包命令

#### 标准打包（推荐）
```bash
# 完整打包（包含前端构建 + Rust 编译）
pnpm build

# 使用本地配置打包
pnpm build:local

# 快速打包（使用 fast-release profile，编译更快但性能较低）
pnpm build:fast
pnpm build:local:fast
```

#### 指定平台打包
```bash
# Windows x64
pnpm build -- --target x86_64-pc-windows-msvc

# Windows x86
pnpm build -- --target i686-pc-windows-msvc

# Windows ARM64
pnpm build -- --target aarch64-pc-windows-msvc
```

#### 使用特定配置文件
```bash
# 使用 Windows 配置
pnpm build -- -c src-tauri/tauri.windows.conf.json

# 使用本地配置
pnpm build:local
```

### 打包输出

打包完成后，产物位于：

```
src-tauri/target/release/bundle/
├── msi/                    # Windows MSI 安装包
│   └── Clash Verge Optimized_0.0.3_x64_en-US.msi
├── nsis/                   # Windows NSIS 安装包
│   └── Clash Verge Optimized_0.0.3_x64-setup.exe
└── updater/                # 更新器文件
    └── Clash Verge Optimized_0.0.3_x64.msi.zip
```

---

## 🤖 CI/CD 自动打包

### GitHub Actions 工作流

项目使用 GitHub Actions 自动打包，配置文件：`.github/workflows/release.yml`

### 触发条件

推送以下 tag 时自动触发打包：

```bash
# 正式版本
git tag v1.2.3
git push origin v1.2.3

# 预发布版本
git tag alpha
git tag beta
git tag rc
git tag pre
git push origin alpha
```

### 打包矩阵

CI/CD 会自动打包以下平台：

#### Windows Standard（标准版）
- x64 (x86_64-pc-windows-msvc)
- x86 (i686-pc-windows-msvc)
- ARM64 (aarch64-pc-windows-msvc)

#### Windows Fixed WebView2（内置 WebView2）
- x64 (使用 webview2.x64.json)
- x86 (使用 webview2.x86.json)
- ARM64 (使用 webview2.arm64.json)

### 发布流程

1. **构建阶段**
   - 安装依赖（pnpm install）
   - 准备资源（pnpm prebuild）
   - 编译打包（tauri build）
   - 签名（使用 TAURI_SIGNING_PRIVATE_KEY）

2. **发布阶段**
   - 上传到 GitHub Releases
   - 生成更新器清单（updater manifests）
   - 发布正式版或预发布版

### 环境变量

需要在 GitHub Secrets 中配置：

- `GITHUB_TOKEN`: 自动提供，用于上传 Release
- `TAURI_SIGNING_PRIVATE_KEY`: Tauri 签名私钥
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: 签名私钥密码

---

## 🔄 版本号同步

### 手动同步

如果需要手动修改版本号，必须同步更新以下文件：

1. `package.json` → `"version": "x.x.x"`
2. `src-tauri/Cargo.toml` → `version = "x.x.x"`
3. `src-tauri/tauri.conf.json` → `"version": "x.x.x"`

### 自动同步（推荐）

使用脚本自动同步：

```bash
# 更新到新版本
pnpm release-version 1.2.3

# 脚本会自动更新所有 3 个文件
```

### 验证同步

```bash
# 检查 package.json
cat package.json | grep version

# 检查 Cargo.toml
cat src-tauri/Cargo.toml | grep version

# 检查 tauri.conf.json
cat src-tauri/tauri.conf.json | grep version
```

---

## 🔧 打包流程详解

### 1. 前端构建

```bash
# 由 tauri.conf.json 中的 beforeBuildCommand 自动执行
pnpm run web:build

# 等价于
tsc --noEmit && vite build
```

输出目录：`dist/`

### 2. 资源准备

```bash
pnpm prebuild
```

下载和准备以下资源：

- **verge-mihomo**: Clash Meta 核心（本地管理，不自动下载）
- **clash-verge-service**: 系统服务（自动下载最新版）
- **Country.mmdb**: GeoIP 数据库
- **geosite.dat**: GeoSite 数据库
- **geoip.dat**: GeoIP 数据库
- **enableLoopback.exe**: UWP 回环工具（Windows）
- **SimpleSC.dll**: NSIS 服务插件（Windows）

资源位置：
- `src-tauri/sidecar/`: 可执行文件（mihomo, service）
- `src-tauri/resources/`: 数据文件（mmdb, dat）

### 3. Rust 编译

```bash
# 由 tauri build 自动执行
cargo build --release
```

编译配置：
- **Profile**: `release` (Cargo.toml)
  - `opt-level = 3`: 最高优化
  - `lto = "thin"`: 链接时优化
  - `codegen-units = 1`: 单个代码生成单元
  - `strip = "none"`: 保留调试符号

- **Profile**: `fast-release` (快速构建)
  - `opt-level = 0`: 无优化
  - `lto = false`: 禁用 LTO
  - `codegen-units = 64`: 并行编译

### 4. 打包

Tauri 自动打包为：
- **Windows**: MSI, NSIS, Portable
- **macOS**: DMG, App Bundle
- **Linux**: AppImage, Deb, RPM

### 5. 签名

使用 Tauri 内置签名机制：

```bash
# 本地签名（从 ~/.tauri/clash-verge-optimized/updater.key 读取）
pnpm build

# CI/CD 签名（从环境变量读取）
TAURI_SIGNING_PRIVATE_KEY=xxx pnpm build
```

---

## 🛠️ 常见问题

### 1. 版本号不一致

**问题**: 打包后版本号显示不正确

**解决**:
```bash
# 使用脚本统一更新
pnpm release-version 1.2.3

# 验证
grep -r "\"version\"" package.json src-tauri/tauri.conf.json
grep "^version" src-tauri/Cargo.toml
```

### 2. 资源文件缺失

**问题**: 打包时提示缺少 mihomo 或其他资源

**解决**:
```bash
# 重新下载所有资源
pnpm prebuild --force

# 检查资源目录
ls -la src-tauri/sidecar/
ls -la src-tauri/resources/
```

### 3. 签名失败

**问题**: 打包时签名失败

**解决**:
```bash
# 检查签名密钥
ls ~/.tauri/clash-verge-optimized/updater.key

# 如果没有密钥，生成新密钥
tauri signer generate -w ~/.tauri/clash-verge-optimized/updater.key
```

### 4. 编译速度慢

**问题**: Rust 编译时间过长

**解决**:
```bash
# 使用快速构建 profile
pnpm build:fast

# 或者使用增量编译（开发时）
pnpm dev
```

### 5. 跨平台打包

**问题**: 在 Windows 上打包 macOS/Linux 版本

**解决**:
- 使用 GitHub Actions CI/CD 自动打包所有平台
- 或者使用 Docker 容器进行跨平台编译

### 6. WebView2 问题

**问题**: 用户系统没有 WebView2

**解决**:
```bash
# 打包内置 WebView2 版本
pnpm build -- -c src-tauri/webview2.x64.json
```

---

## 📝 发布检查清单

发布新版本前的检查清单：

- [ ] 更新版本号（使用 `pnpm release-version`）
- [ ] 验证版本号同步（package.json, Cargo.toml, tauri.conf.json）
- [ ] 运行类型检查（`pnpm typecheck`）
- [ ] 运行 Rust 检查（`cargo check`）
- [ ] 本地测试打包（`pnpm build`）
- [ ] 测试打包产物是否正常运行
- [ ] 更新 CHANGELOG.md
- [ ] 提交代码并推送
- [ ] 创建并推送 tag（`git tag v1.2.3 && git push origin v1.2.3`）
- [ ] 等待 CI/CD 完成打包
- [ ] 验证 GitHub Release 中的文件
- [ ] 测试自动更新功能

---

## 🎯 快速参考

### 常用命令

```bash
# 开发
pnpm dev                    # 启动开发服务器

# 类型检查
pnpm typecheck              # TypeScript 类型检查
cargo check                 # Rust 类型检查

# 打包
pnpm prebuild               # 准备资源
pnpm build                  # 完整打包
pnpm build:fast             # 快速打包

# 版本管理
pnpm release-version 1.2.3  # 更新版本号
pnpm release-version beta   # 预发布版本
pnpm release-version autobuild  # 自动构建版本

# 发布
git tag v1.2.3              # 创建 tag
git push origin v1.2.3      # 推送 tag（触发 CI/CD）
```

### 版本号规则

- **正式版**: `v1.2.3` → 稳定版本
- **预发布**: `v1.2.3-beta` → 测试版本
- **自动构建**: `v1.2.3+autobuild.1004.cc39b27` → 每日构建
- **部署测试**: `v1.2.3+deploytest.1004.cc39b27` → 部署测试

### 文件位置

```
项目根目录/
├── package.json                    # 前端版本号
├── src-tauri/
│   ├── Cargo.toml                  # Rust 版本号
│   ├── tauri.conf.json             # Tauri 版本号
│   ├── sidecar/                    # 可执行文件
│   │   └── verge-mihomo-*          # Mihomo 核心
│   ├── resources/                  # 资源文件
│   │   ├── Country.mmdb
│   │   ├── geosite.dat
│   │   └── geoip.dat
│   └── target/release/bundle/      # 打包输出
├── scripts/
│   ├── tauri-build.mjs             # 打包脚本
│   ├── release-version.mjs         # 版本管理脚本
│   └── prebuild.mjs                # 资源准备脚本
└── .github/workflows/
    └── release.yml                 # CI/CD 配置
```

---

**最后更新**: 2026-05-27  
**当前版本**: 0.0.3
