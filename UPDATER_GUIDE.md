# Tauri 自动更新指南

## 概述

`.sig` 文件是 Tauri 自动更新系统的**数字签名文件**，用于验证更新包的完整性和真实性，防止恶意篡改。

## 文件说明

### 生成的文件

构建完成后，在 `target/release/bundle/nsis/` 目录下会生成：

```
Clash Verge Optimized_0.0.3_x64-setup.exe       # 安装包
Clash Verge Optimized_0.0.3_x64-setup.exe.sig   # 签名文件（用于自动更新）
```

### 签名密钥

项目使用的签名密钥位置：
- **私钥**：`~/.tauri/clash-verge-optimized/updater.key`
- **公钥**：已配置在 `src-tauri/tauri.conf.json` 中

```json
{
  "plugins": {
    "updater": {
      "active": true,
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQ2NjJBMUZGRDhFN0M5REIKUldUYnllZlkvNkZpUms0anNHcktTbm1BNGtITnM1OUxDaTh0aGxyVk9CeW4zbmFybGMvOW85WVAK"
    }
  }
}
```

## 自动更新工作流程

### 1. 构建阶段

```bash
pnpm build
```

- 构建时，`tauri-build.mjs` 会自动加载私钥
- Tauri 使用私钥对安装包生成 `.sig` 签名文件
- 配置项 `"createUpdaterArtifacts": true` 启用签名生成

### 2. 发布阶段

需要将以下文件上传到 GitHub Release：

```
Clash Verge Optimized_0.0.3_x64-setup.exe
Clash Verge Optimized_0.0.3_x64-setup.exe.sig
```

### 3. 创建更新清单

在 GitHub Release 中创建一个 `update.json` 文件（标签：`updater`）：

```json
{
  "version": "0.0.3",
  "notes": "更新说明",
  "pub_date": "2026-05-27T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "从 .sig 文件中读取的内容",
      "url": "https://github.com/tanzanite2025/clash-verge-optimized/releases/download/v0.0.3/Clash Verge Optimized_0.0.3_x64-setup.exe"
    }
  }
}
```

### 4. 客户端检查更新

应用启动时：
1. 从配置的 endpoints 获取 `update.json`
2. 比较版本号
3. 如果有新版本，下载安装包和 `.sig` 文件
4. 使用内置的公钥验证签名
5. 验证通过后执行更新

## 更新端点配置

当前配置了两个端点（带代理和不带代理）：

```json
"endpoints": [
  "https://update.hwdns.net/https://github.com/tanzanite2025/clash-verge-optimized/releases/download/updater/update-proxy.json",
  "https://github.com/tanzanite2025/clash-verge-optimized/releases/download/updater/update.json"
]
```

## 发布新版本的步骤

### 方法 1：手动发布

1. **构建应用**
   ```bash
   pnpm release-version 0.0.3
   pnpm build
   ```

2. **创建 GitHub Release**
   - 标签：`v0.0.3`
   - 上传文件：
     - `Clash Verge Optimized_0.0.3_x64-setup.exe`
     - `Clash Verge Optimized_0.0.3_x64-setup.exe.sig`

3. **生成签名内容**
   ```bash
   # 读取 .sig 文件内容（Base64 编码）
   cat "target/release/bundle/nsis/Clash Verge Optimized_0.0.3_x64-setup.exe.sig"
   ```

4. **创建/更新 update.json**
   - 在 `updater` 标签的 Release 中创建 `update.json`
   - 填入版本信息、下载链接和签名

5. **（可选）创建 update-proxy.json**
   - 内容与 `update.json` 相同
   - 用于国内用户加速访问

### 方法 2：使用 GitHub Actions（推荐）

项目已配置 `.github/workflows/release.yml`，可以自动化发布流程。

查看该文件了解自动化配置。

## 安全注意事项

### 保护私钥

⚠️ **私钥必须保密！**

- 私钥位置：`~/.tauri/clash-verge-optimized/updater.key`
- **绝对不要**提交到 Git 仓库
- **绝对不要**公开分享
- 如果私钥泄露，需要重新生成密钥对并更新所有客户端

### 生成新的密钥对

如果需要重新生成密钥对：

```bash
# 安装 tauri-cli
pnpm add -D @tauri-apps/cli

# 生成新密钥对
pnpm tauri signer generate -w ~/.tauri/clash-verge-optimized/updater.key

# 会输出公钥，需要更新到 tauri.conf.json 的 pubkey 字段
```

### GitHub Actions 配置

如果使用 GitHub Actions 自动构建，需要：

1. 将私钥内容添加到 GitHub Secrets
   - 名称：`TAURI_SIGNING_PRIVATE_KEY`
   - 值：私钥文件的完整内容

2. 在 workflow 中设置环境变量：
   ```yaml
   env:
     TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
   ```

## 验证更新配置

### 测试更新检查

在应用中添加测试代码：

```typescript
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

async function checkForUpdates() {
  const update = await check()
  
  if (update?.available) {
    console.log(`发现新版本: ${update.version}`)
    console.log(`当前版本: ${update.currentVersion}`)
    
    // 下载并安装更新
    await update.downloadAndInstall()
    
    // 重启应用
    await relaunch()
  } else {
    console.log('已是最新版本')
  }
}
```

### 手动验证签名

```bash
# 使用 minisign 验证签名（需要安装 minisign）
minisign -Vm "Clash Verge Optimized_0.0.3_x64-setup.exe" -P <公钥>
```

## 常见问题

### Q: 为什么需要 .sig 文件？

A: 防止中间人攻击。如果没有签名验证，攻击者可能会劫持更新请求，提供恶意的安装包。

### Q: 可以不使用签名吗？

A: 技术上可以，但**强烈不推荐**。这会让用户面临安全风险。

### Q: 签名验证失败怎么办？

A: 检查：
1. 公钥是否正确配置在 `tauri.conf.json`
2. `.sig` 文件是否与安装包匹配
3. 私钥是否在构建时正确加载

### Q: 如何禁用自动更新？

A: 在 `tauri.conf.json` 中设置：
```json
{
  "plugins": {
    "updater": {
      "active": false
    }
  }
}
```

## 相关文档

- [Tauri 更新器官方文档](https://v2.tauri.app/plugin/updater/)
- [Minisign 签名工具](https://jedisct1.github.io/minisign/)
- 项目构建脚本：`scripts/tauri-build.mjs`
- 项目配置文件：`src-tauri/tauri.conf.json`

---

**最后更新：** 2026-05-27  
**当前版本：** 0.0.3
