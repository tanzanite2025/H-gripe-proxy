# 跨平台构建指南

## 当前状态

### ✅ 支持的平台（本地 + CI/CD）
- **Windows x64** - 标准版 + Fixed WebView2 版
- **Windows x86** - 标准版 + Fixed WebView2 版
- **Windows ARM64** - 标准版 + Fixed WebView2 版

### ❌ 不支持的平台（需要添加）
- **macOS Intel (x64)**
- **macOS Apple Silicon (ARM64)**
- **Linux x64**
- **Linux ARM64**

## 为什么本地打包只有 Windows 版本？

### 跨平台编译的限制

**在 Windows 上只能编译 Windows 版本：**
```
Windows 系统 → ✅ Windows .exe/.msi
Windows 系统 → ❌ macOS .dmg/.app (需要 macOS + Xcode)
Windows 系统 → ❌ Linux .deb/.AppImage (需要 Linux)
```

**在 macOS 上只能编译 macOS 版本：**
```
macOS 系统 → ✅ macOS .dmg/.app
macOS 系统 → ❌ Windows .exe/.msi
macOS 系统 → ❌ Linux .deb/.AppImage
```

**在 Linux 上只能编译 Linux 版本：**
```
Linux 系统 → ✅ Linux .deb/.AppImage
Linux 系统 → ❌ Windows .exe/.msi
Linux 系统 → ❌ macOS .dmg/.app
```

### 为什么分析软件说支持苹果？

1. **图标文件存在** - `src-tauri/icons/icon.icns`（macOS 图标）
2. **Tauri 框架支持** - Tauri 本身是跨平台框架
3. **代码中有条件编译** - 例如 `#[cfg(target_os = "macos")]`

但这**不代表你能在 Windows 上打包出 macOS 版本**！

## 解决方案

### 方案 1：使用 GitHub Actions（推荐）

通过 CI/CD 在云端构建所有平台版本。

#### 添加 macOS 构建

在 `.github/workflows/release.yml` 中添加：

```yaml
  build-macos:
    name: macOS (${{ matrix.label }})
    runs-on: macos-latest
    strategy:
      fail-fast: false
      matrix:
        include:
          - label: Intel
            rust_target: x86_64-apple-darwin
            tauri_args: --target x86_64-apple-darwin
          - label: Apple Silicon
            rust_target: aarch64-apple-darwin
            tauri_args: --target aarch64-apple-darwin
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 10.33.0

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: pnpm-lock.yaml

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.95.0
          targets: ${{ matrix.rust_target }}

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Prepare bundled resources
        run: pnpm prebuild

      - name: Build and upload signed release assets
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: Clash Verge Optimized ${{ github.ref_name }}
          releaseBody: Signed release assets for Clash Verge Optimized.
          releaseDraft: false
          prerelease: ${{ contains(github.ref_name, '-') || contains(github.ref_name, '+') || github.ref_name == 'alpha' || github.ref_name == 'beta' || github.ref_name == 'rc' || github.ref_name == 'pre' }}
          args: ${{ matrix.tauri_args }}
```

#### 添加 Linux 构建

```yaml
  build-linux:
    name: Linux (${{ matrix.label }})
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        include:
          - label: x64
            rust_target: x86_64-unknown-linux-gnu
            tauri_args: --target x86_64-unknown-linux-gnu
          - label: ARM64
            rust_target: aarch64-unknown-linux-gnu
            tauri_args: --target aarch64-unknown-linux-gnu
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 10.33.0

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: pnpm-lock.yaml

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.95.0
          targets: ${{ matrix.rust_target }}

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Prepare bundled resources
        run: pnpm prebuild

      - name: Build and upload signed release assets
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: Clash Verge Optimized ${{ github.ref_name }}
          releaseBody: Signed release assets for Clash Verge Optimized.
          releaseDraft: false
          prerelease: ${{ contains(github.ref_name, '-') || contains(github.ref_name, '+') || github.ref_name == 'alpha' || github.ref_name == 'beta' || github.ref_name == 'rc' || github.ref_name == 'pre' }}
          args: ${{ matrix.tauri_args }}
```

#### 更新 updater manifests 依赖

```yaml
  publish-updater-manifests:
    name: Publish Updater Manifests
    runs-on: ubuntu-latest
    needs:
      - build-windows-standard
      - build-windows-fixed-webview2
      - build-macos  # ✅ 新增
      - build-linux  # ✅ 新增
```

### 方案 2：本地多平台构建（不推荐）

需要准备 3 台不同系统的机器：
1. **Windows 机器** - 构建 Windows 版本
2. **macOS 机器** - 构建 macOS 版本
3. **Linux 机器** - 构建 Linux 版本

然后手动合并所有构建产物。

**缺点：**
- 需要多台机器
- 手动操作容易出错
- 无法自动化

### 方案 3：使用虚拟机（不推荐）

在 Windows 上运行 macOS/Linux 虚拟机，但：
- **macOS 虚拟机** - 违反 Apple EULA（除非在 Mac 硬件上）
- **Linux 虚拟机** - 可行但性能差
- **构建速度慢**

## 推荐流程

### 开发阶段（本地）
```bash
# 只构建当前平台用于测试
pnpm build:fast
```

### 发布阶段（CI/CD）
```bash
# 1. 更新版本号
pnpm release-version

# 2. 提交并打 tag
git add .
git commit -m "chore: release v0.0.5"
git tag v0.0.5
git push origin main --tags

# 3. GitHub Actions 自动构建所有平台
# - Windows x64/x86/ARM64
# - macOS Intel/Apple Silicon
# - Linux x64/ARM64
```

## macOS 代码签名（可选）

如果需要 macOS 代码签名和公证，需要：

1. **Apple Developer 账号**（$99/年）
2. **创建证书**
   - Developer ID Application Certificate
   - Developer ID Installer Certificate

3. **配置 GitHub Secrets**
   ```
   APPLE_CERTIFICATE - Base64 编码的 .p12 证书
   APPLE_CERTIFICATE_PASSWORD - 证书密码
   APPLE_ID - Apple ID 邮箱
   APPLE_PASSWORD - App-specific password
   APPLE_TEAM_ID - Team ID
   ```

4. **更新 tauri.conf.json**
   ```json
   {
     "bundle": {
       "macOS": {
         "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)",
         "entitlements": "path/to/entitlements.plist"
       }
     }
   }
   ```

## Linux 特殊依赖

### XDP 功能（仅 Linux）

如果启用 XDP 功能，需要：
```bash
# 安装 eBPF 开发工具
sudo apt-get install -y \
  clang \
  llvm \
  libelf-dev \
  linux-headers-$(uname -r)

# 构建 XDP 程序
cd crates/clash-verge-xdp
./build.sh
```

### 系统依赖

```bash
# Ubuntu/Debian
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf

# Fedora/RHEL
sudo dnf install -y \
  webkit2gtk4.1-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  patchelf

# Arch Linux
sudo pacman -S \
  webkit2gtk-4.1 \
  libappindicator-gtk3 \
  librsvg \
  patchelf
```

## 构建产物

### Windows
- `Clash.Verge.Optimized_0.0.4_x64_en-US.msi` - 安装包
- `Clash.Verge.Optimized_0.0.4_x64-setup.exe` - 安装程序
- `Clash.Verge.Optimized_0.0.4_x64_en-US.msi.zip` - 签名包
- `Clash.Verge.Optimized_0.0.4_x64_en-US.msi.zip.sig` - 签名

### macOS
- `Clash.Verge.Optimized_0.0.4_x64.dmg` - Intel 磁盘镜像
- `Clash.Verge.Optimized_0.0.4_aarch64.dmg` - Apple Silicon 磁盘镜像
- `Clash.Verge.Optimized_0.0.4_universal.dmg` - 通用二进制（可选）

### Linux
- `clash-verge-optimized_0.0.4_amd64.deb` - Debian/Ubuntu 包
- `clash-verge-optimized_0.0.4_amd64.AppImage` - AppImage
- `clash-verge-optimized-0.0.4-1.x86_64.rpm` - Fedora/RHEL 包（可选）

## 常见问题

### Q: 为什么我本地打包只有 Windows 版本？
**A:** 这是正常的！跨平台编译有系统限制，需要在对应系统上构建。使用 GitHub Actions 可以自动构建所有平台。

### Q: 如何测试 macOS/Linux 版本？
**A:** 
1. 使用 GitHub Actions 构建
2. 下载对应平台的构建产物
3. 在对应系统上测试

### Q: 是否必须支持所有平台？
**A:** 不是。可以只发布 Windows 版本，根据用户需求逐步添加其他平台。

### Q: macOS 代码签名是否必须？
**A:** 不是必须，但强烈推荐：
- ✅ 有签名 - 用户可以直接打开，无警告
- ❌ 无签名 - 用户需要右键 → 打开，或在系统设置中允许

### Q: Linux 是否需要打包成多种格式？
**A:** 推荐至少提供：
- `.deb` - Debian/Ubuntu 用户
- `.AppImage` - 通用格式，无需安装

## 总结

- **本地开发** - 只构建当前平台，快速测试
- **正式发布** - 使用 GitHub Actions 构建所有平台
- **Windows 优先** - 当前已完善，可先发布 Windows 版本
- **逐步扩展** - 根据用户需求添加 macOS/Linux 支持

如果只是个人使用或小范围分享，**只构建 Windows 版本完全足够**！
