# 图标更新指南

## 问题描述

`target/release/` 下生成的可执行文件使用的是旧项目的图标，而不是新的图标。

## 原因分析

### Windows 可执行文件图标的工作原理

1. **编译时嵌入**：图标在 Rust 编译时被嵌入到 `.exe` 文件中
2. **不会自动更新**：已经编译好的 `.exe` 文件不会因为图标文件更新而改变
3. **需要重新构建**：必须重新编译才能应用新图标

### 图标配置位置

**配置文件：** `src-tauri/tauri.conf.json`

```json
{
  "bundle": {
    "icon": [
      "icons/32x32.png",      // macOS 状态栏图标
      "icons/128x128.png",    // macOS 应用图标
      "icons/128x128@2x.png", // macOS Retina 图标
      "icons/icon.icns",      // macOS 应用图标包
      "icons/icon.ico"        // Windows 可执行文件图标 ← 这个！
    ]
  }
}
```

**图标文件位置：** `src-tauri/icons/`

```
src-tauri/icons/
├── icon.ico           ← Windows 可执行文件图标（主要）
├── 32x32.png          ← macOS 状态栏
├── 128x128.png        ← macOS 应用
├── 128x128@2x.png     ← macOS Retina
├── icon.icns          ← macOS 应用包
├── icon.png           ← 源图标
├── tray-icon.ico      ← 系统托盘图标（正常）
├── tray-icon-sys.ico  ← 系统托盘图标（系统代理）
└── tray-icon-tun.ico  ← 系统托盘图标（TUN 模式）
```

## 解决方案

### 方法 1：重新构建（推荐）

如果图标文件已经是新的，只需重新构建：

```bash
# 清理旧的构建产物
cargo clean

# 重新构建
pnpm build
```

新生成的 `clash-verge-optimized.exe` 将使用新图标。

### 方法 2：替换图标文件

如果 `src-tauri/icons/icon.ico` 确实是旧图标，需要替换：

#### 步骤 1：准备新图标

你需要一个 `.ico` 文件，包含多个尺寸：
- 16x16
- 32x32
- 48x48
- 64x64
- 128x128
- 256x256

#### 步骤 2：生成 .ico 文件

**选项 A：使用在线工具**
- [ICO Convert](https://icoconvert.com/)
- [Favicon.io](https://favicon.io/)
- [RealFaviconGenerator](https://realfavicongenerator.net/)

**选项 B：使用 ImageMagick**
```bash
magick convert icon.png -define icon:auto-resize=256,128,64,48,32,16 icon.ico
```

**选项 C：使用 Tauri CLI**
```bash
pnpm tauri icon path/to/your/icon.png
```

这会自动生成所有需要的图标文件到 `src-tauri/icons/`。

#### 步骤 3：替换文件

将新的 `icon.ico` 复制到 `src-tauri/icons/icon.ico`。

#### 步骤 4：重新构建

```bash
cargo clean
pnpm build
```

### 方法 3：使用 Tauri Icon 命令（最简单）

如果你有一个高质量的 PNG 图标（推荐 1024x1024 或更大）：

```bash
# 自动生成所有平台的图标
pnpm tauri icon path/to/your/icon.png

# 或者如果图标在项目根目录
pnpm tauri icon icon.png
```

这会自动生成：
- `icon.ico` - Windows 图标
- `icon.icns` - macOS 图标
- `32x32.png`, `128x128.png`, `128x128@2x.png` - 各种尺寸

然后重新构建：

```bash
pnpm build
```

## 验证图标

### 1. 检查图标文件

在 Windows 资源管理器中：
1. 导航到 `src-tauri\icons\`
2. 右键点击 `icon.ico`
3. 选择"打开方式" → "画图"或其他图片查看器
4. 确认是否是你想要的图标

### 2. 检查可执行文件图标

构建完成后：
1. 导航到 `target\release\`
2. 查看 `clash-verge-optimized.exe` 的图标
3. 如果图标正确，说明更新成功

### 3. 检查安装包图标

1. 导航到 `target\release\bundle\nsis\`
2. 查看 `Clash Verge Optimized_0.0.3_x64-setup.exe` 的图标
3. 安装后检查桌面快捷方式和开始菜单的图标

## 图标规范

### Windows (.ico)

**推荐尺寸：**
- 16x16 - 小图标（任务栏、文件列表）
- 32x32 - 中等图标（桌面、文件夹）
- 48x48 - 大图标（大图标视图）
- 256x256 - 超大图标（Windows 7+）

**格式要求：**
- 文件格式：ICO
- 颜色深度：32-bit（带透明通道）
- 包含多个尺寸

### macOS (.icns)

**推荐尺寸：**
- 16x16, 32x32, 64x64, 128x128, 256x256, 512x512, 1024x1024

**格式要求：**
- 文件格式：ICNS
- 颜色深度：32-bit
- 支持 Retina 显示

### 托盘图标

**Windows 托盘图标：**
- 尺寸：16x16 或 32x32
- 格式：ICO
- 建议使用简单、高对比度的设计
- 支持透明背景

**文件：**
- `tray-icon.ico` - 默认状态
- `tray-icon-sys.ico` - 系统代理模式
- `tray-icon-tun.ico` - TUN 模式

## 常见问题

### Q: 为什么更新了图标文件，但可执行文件图标没变？

A: 图标在编译时嵌入，必须重新构建。运行 `cargo clean` 然后 `pnpm build`。

### Q: 为什么安装包图标正确，但可执行文件图标不对？

A: 可能是 Windows 图标缓存问题。尝试：
1. 重启 Windows 资源管理器
2. 清理图标缓存：
   ```bash
   ie4uinit.exe -show
   ```

### Q: 如何制作高质量的图标？

A: 建议：
1. 从矢量图（SVG）或高分辨率 PNG（1024x1024+）开始
2. 使用专业工具（如 Adobe Illustrator、Figma）
3. 确保在小尺寸（16x16）下仍然清晰可辨
4. 使用简单、高对比度的设计

### Q: 托盘图标和应用图标有什么区别？

A: 
- **应用图标**（`icon.ico`）：用于可执行文件、快捷方式、任务栏
- **托盘图标**（`tray-icon.ico`）：用于系统托盘，通常更简单、更小

### Q: 可以使用 PNG 作为 Windows 图标吗？

A: 不可以。Windows 可执行文件必须使用 `.ico` 格式。但你可以：
1. 从 PNG 生成 ICO
2. 使用 `pnpm tauri icon` 自动转换

## 图标设计建议

### 1. 保持简单

- 避免过多细节
- 在小尺寸（16x16）下仍然清晰
- 使用高对比度

### 2. 品牌一致性

- 与应用主题颜色一致
- 与品牌标识一致
- 跨平台保持一致

### 3. 适配不同背景

- 测试浅色和深色背景
- 使用透明背景
- 考虑添加轮廓或阴影

### 4. 多尺寸优化

- 为每个尺寸单独优化
- 小尺寸时简化细节
- 大尺寸时增加细节

## 相关文档

- [Tauri 图标指南](https://v2.tauri.app/develop/visual/)
- [Windows 图标设计指南](https://learn.microsoft.com/en-us/windows/apps/design/style/iconography/app-icon-design)
- [macOS 图标设计指南](https://developer.apple.com/design/human-interface-guidelines/app-icons)

## 总结

✅ **图标更新步骤**
1. 准备高质量的源图标（PNG, 1024x1024+）
2. 运行 `pnpm tauri icon icon.png` 生成所有图标
3. 运行 `cargo clean` 清理旧构建
4. 运行 `pnpm build` 重新构建
5. 验证新图标

⚠️ **重要提醒**
- 图标在编译时嵌入，必须重新构建
- 已编译的 `.exe` 文件不会自动更新图标
- 清理构建缓存可以避免使用旧图标

---

**最后更新：** 2026-05-27  
**当前图标位置：** `src-tauri/icons/icon.ico`  
**下次构建后生效**
