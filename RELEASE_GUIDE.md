# 发布新版本指南

## 快速发布流程

### 1. 更新版本号

```bash
pnpm release-version 0.0.4
```

这会自动更新：
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

### 2. 提交更改

```bash
git add .
git commit -m "chore: bump version to 0.0.4"
```

### 3. 创建并推送标签

```bash
git tag v0.0.4
git push origin main
git push origin v0.0.4
```

### 4. 自动构建和发布

推送标签后，GitHub Actions 会自动：

1. ✅ 构建 Windows 版本（x64, x86, arm64）
2. ✅ 构建固定 WebView2 版本（x64, x86, arm64）
3. ✅ 生成 `.sig` 签名文件
4. ✅ 创建 GitHub Release
5. ✅ 上传所有安装包和签名文件
6. ✅ 生成并发布 `update.json` 和 `update-proxy.json`

**整个过程完全自动化，无需手动操作！**

## .sig 文件的作用

### 什么是 .sig 文件？

`.sig` 文件是**数字签名文件**，用于：
- ✅ 验证安装包的完整性（未被篡改）
- ✅ 验证安装包的真实性（来自官方）
- ✅ 防止中间人攻击

### 工作原理

```
构建时：
  安装包 + 私钥 → 生成 .sig 签名文件

更新时：
  安装包 + .sig 文件 + 公钥 → 验证通过 → 安装更新
                              ↓
                          验证失败 → 拒绝更新
```

### 签名密钥位置

- **私钥**（保密）：`~/.tauri/clash-verge-optimized/updater.key`
- **公钥**（公开）：已配置在 `src-tauri/tauri.conf.json`

⚠️ **重要**：私钥必须保密，已添加到 GitHub Secrets 中供 CI 使用。

## 自动更新流程

### 用户端

1. 应用启动时检查更新
2. 从以下端点获取 `update.json`：
   - 主端点：`https://github.com/.../updater/update.json`
   - 代理端点：`https://update.hwdns.net/...` (国内加速)
3. 比较版本号
4. 下载新版本安装包和 `.sig` 文件
5. 使用内置公钥验证签名
6. 验证通过后安装更新

### 更新清单示例

GitHub Actions 会自动生成 `update.json`：

```json
{
  "version": "0.0.4",
  "notes": "更新说明",
  "pub_date": "2026-05-27T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "自动从 .sig 文件读取",
      "url": "https://github.com/.../Clash_Verge_Optimized_0.0.4_x64-setup.exe"
    }
  }
}
```

## 版本类型

### 稳定版本

```bash
git tag v1.0.0    # 格式：v主版本.次版本.修订版本
```

- 触发完整构建（标准版 + 固定 WebView2 版）
- 生成两套更新清单

### 预发布版本

```bash
git tag v1.0.0-beta.1    # 带 - 或 + 的版本
git tag alpha            # 特殊标签
git tag beta
git tag rc
git tag pre
```

- 标记为预发布版本
- 仅生成标准版更新清单

## 本地构建（可选）

如果需要本地构建测试：

```bash
# 完整构建（包含 Rust 编译，需要 15-20 分钟）
pnpm build

# 仅构建前端（快速测试）
pnpm run web:build

# 生成便携版
pnpm portable
```

构建产物位置：
- 安装包：`target/release/bundle/nsis/`
- 便携版：`target/release/bundle/portable/`

## 检查构建状态

1. 访问 GitHub Actions：
   ```
   https://github.com/tanzanite2025/clash-verge-optimized/actions
   ```

2. 查看 Release 工作流状态

3. 构建完成后，检查 Release 页面：
   ```
   https://github.com/tanzanite2025/clash-verge-optimized/releases
   ```

## 常见问题

### Q: 为什么需要 GitHub Secrets？

A: 私钥用于签名，必须保密。GitHub Actions 需要通过 Secrets 安全地访问私钥。

需要配置的 Secrets：
- `TAURI_SIGNING_PRIVATE_KEY`：私钥内容
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥密码（如果有）

### Q: 如何添加 GitHub Secrets？

1. 进入仓库 Settings → Secrets and variables → Actions
2. 点击 "New repository secret"
3. 添加密钥：
   - Name: `TAURI_SIGNING_PRIVATE_KEY`
   - Value: 复制 `~/.tauri/clash-verge-optimized/updater.key` 的完整内容

### Q: 构建失败怎么办？

1. 检查 GitHub Actions 日志
2. 常见问题：
   - 私钥未配置或错误
   - 依赖安装失败
   - Rust 编译错误
   - 版本号格式错误

### Q: 如何测试自动更新？

1. 安装旧版本应用
2. 发布新版本
3. 启动应用，应该会提示更新
4. 检查日志确认更新流程

### Q: 可以手动上传 .sig 文件吗？

A: 可以，但不推荐。GitHub Actions 已经自动化了整个流程，手动操作容易出错。

## 版本号规范

遵循 [语义化版本](https://semver.org/lang/zh-CN/)：

- **主版本号**：不兼容的 API 修改
- **次版本号**：向下兼容的功能性新增
- **修订号**：向下兼容的问题修正

示例：
- `v1.0.0` - 首个稳定版
- `v1.1.0` - 新增功能
- `v1.1.1` - 修复 bug
- `v2.0.0` - 重大更新（可能不兼容）
- `v1.2.0-beta.1` - 测试版

## 相关文件

- **构建配置**：`.github/workflows/release.yml`
- **更新配置**：`src-tauri/tauri.conf.json`
- **构建脚本**：`scripts/tauri-build.mjs`
- **版本脚本**：`scripts/release-version.mjs`
- **更新脚本**：`scripts/updater.mjs`（如果存在）

## 更多信息

详细的签名和更新机制说明，请参考：
- [UPDATER_GUIDE.md](./UPDATER_GUIDE.md) - 自动更新详细指南
- [Tauri 更新器文档](https://v2.tauri.app/plugin/updater/)

---

**最后更新：** 2026-05-27  
**当前版本：** 0.0.3  
**下一个版本：** 0.0.4（示例）
