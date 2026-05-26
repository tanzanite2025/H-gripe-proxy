# 包名称修复说明

## 问题描述

在 `target/release/` 目录下生成的文件名是旧项目的名称：
- `clash-verge.exe`
- `clash-verge.d`
- `clash-verge.pdb`

而不是期望的 `clash-verge-optimized`。

## 原因分析

项目中有两个不同的名称配置：

### 1. Rust 包名（Cargo.toml）

**文件：** `src-tauri/Cargo.toml`

```toml
[package]
name = "clash-verge"  ← 决定 target/release/ 下的文件名
```

这个名称决定了：
- `target/release/clash-verge.exe` - 可执行文件
- `target/release/clash-verge.d` - 依赖信息
- `target/release/clash-verge.pdb` - 调试符号（Windows）

### 2. 产品名称（tauri.conf.json）

**文件：** `src-tauri/tauri.conf.json`

```json
{
  "productName": "Clash Verge Optimized"  ← 决定安装包名称
}
```

这个名称决定了：
- `Clash Verge Optimized_0.0.3_x64-setup.exe` - 安装包
- 应用程序显示名称
- 开始菜单中的名称

## 解决方案

### 已完成的修改

修改了 `src-tauri/Cargo.toml`：

```toml
[package]
name = "clash-verge-optimized"  ← 新名称
default-run = "clash-verge-optimized"
```

### 影响

下次构建后，`target/release/` 下的文件将变为：
- `clash-verge-optimized.exe`
- `clash-verge-optimized.d`
- `clash-verge-optimized.pdb`

### 需要清理旧文件

旧的构建产物不会自动删除，建议清理：

```bash
# 清理所有构建产物
cargo clean

# 或者手动删除旧文件
Remove-Item target\release\clash-verge.* -Force
```

## 文件说明

### target/release/ 目录结构

```
target/release/
├── clash-verge-optimized.exe     # 主可执行文件
├── clash-verge-optimized.d       # 依赖信息文件
├── clash-verge-optimized.pdb     # 调试符号（Windows）
├── build/                         # 构建脚本输出
├── deps/                          # 依赖库
├── examples/                      # 示例程序
├── incremental/                   # 增量编译缓存
└── bundle/                        # 打包输出
    ├── nsis/                      # NSIS 安装包
    │   ├── Clash Verge Optimized_0.0.3_x64-setup.exe
    │   └── Clash Verge Optimized_0.0.3_x64-setup.exe.sig
    └── msi/                       # MSI 安装包（如果启用）
```

### 哪些文件会被分发？

**会分发：**
- ✅ `bundle/nsis/*.exe` - 安装包
- ✅ `bundle/nsis/*.sig` - 签名文件

**不会分发：**
- ❌ `clash-verge-optimized.exe` - 中间产物
- ❌ `clash-verge-optimized.d` - 构建信息
- ❌ `clash-verge-optimized.pdb` - 调试符号
- ❌ `deps/`, `build/`, `incremental/` - 构建缓存

## 为什么有两个名称？

### Rust 生态系统约定

Rust 包名（`Cargo.toml` 中的 `name`）通常使用：
- 小写字母
- 连字符分隔（kebab-case）
- 简短的技术名称

示例：`clash-verge-optimized`

### 产品名称约定

产品名称（`tauri.conf.json` 中的 `productName`）通常使用：
- 首字母大写
- 空格分隔
- 用户友好的名称

示例：`Clash Verge Optimized`

## 验证修改

### 1. 清理旧构建

```bash
cargo clean
```

### 2. 重新构建

```bash
pnpm build
```

### 3. 检查文件名

```bash
# 检查 target/release/ 下的文件
Get-ChildItem target\release\clash-verge-optimized.*

# 检查安装包
Get-ChildItem target\release\bundle\nsis\
```

## 其他相关配置

### package.json

```json
{
  "name": "clash-verge",  ← 这个是 npm 包名，不影响构建产物
  "version": "0.0.3"
}
```

这个名称只影响：
- `node_modules` 中的包名
- `package-lock.json` 中的引用
- 不影响 Rust 构建或 Tauri 打包

### 建议

如果想保持一致性，可以考虑也修改 `package.json`：

```json
{
  "name": "clash-verge-optimized",
  "version": "0.0.3"
}
```

但这不是必需的，因为前端包名不会影响最终产物。

## 总结

✅ **已修复 Rust 包名**
- 从 `clash-verge` 改为 `clash-verge-optimized`
- 下次构建后，`target/release/` 下的文件名将更新

📦 **产品名称保持不变**
- `Clash Verge Optimized` - 用户看到的名称
- 安装包名称不受影响

🧹 **建议清理**
- 运行 `cargo clean` 清理旧的构建产物
- 删除旧的 `clash-verge.*` 文件

---

**修改时间：** 2026-05-27  
**影响文件：** 1 个文件（`src-tauri/Cargo.toml`）  
**需要操作：** 运行 `cargo clean` 并重新构建
