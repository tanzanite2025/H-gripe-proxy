# Icon Asset Guide

项目根目录：`c:\Users\P16V\Desktop\个人开发\clashverge-clean`

这份文档用于后续**手动替换图标/Logo 素材**。下面列出当前项目里和图标有关的主要文件、路径、尺寸、用途，以及建议怎么替换。

---

## ✅ 图标生成状态（2026-05-27 更新）

**已完成：** 从 `src-tauri/icons/ico.ico` 自动生成了所有 Windows 所需的图标文件！

- ✅ 主程序图标：icon.ico, 32x32.png, 128x128.png, 128x128@2x.png, icon.png
- ✅ 托盘图标：tray-icon.ico, tray-icon-sys.ico, tray-icon-tun.ico
- ⚠️ **待处理**：macOS 的 icon.icns 需要单独生成（见 `ICON_GENERATION_COMPLETE.md`）

**详细报告：** 查看 `ICON_GENERATION_COMPLETE.md`

---

## 1. 前端顶部图标 / Logo

### 1.1 当前实际使用中的前端图标

- **文件**：`src/assets/image/icon_dark.svg`
- **用途**：当前前端顶部栏左上角唯一正在使用的 SVG 图标
- **引用位置**：`src/pages/_layout.tsx`
- **当前画布尺寸**：`viewBox="0 0 48 48"`
- **当前文件大小**：`261 bytes`
- **页面实际显示尺寸**：`22 x 22 px`
- **替换建议**：如果你只想先换前端左上角的小图标，优先替换这个文件

### 1.2 当前保留但未实际使用的前端图标

- **文件**：`src/assets/image/icon_light.svg`
- **用途**：保留文件，当前 `_layout.tsx` 已经不再引用
- **当前画布尺寸**：`viewBox="0 0 48 48"`
- **当前文件大小**：`261 bytes`
- **替换建议**：目前不是必须替换；如果你想给未来预留明色版，也可以同步替换

### 1.3 当前未使用的文字 Logo 占位

- **文件**：`src/assets/image/logo.svg`
- **用途**：目前不再参与顶部栏渲染，仅保留一个空白占位文件
- **当前画布尺寸**：`viewBox="0 0 156 24"`
- **当前文件大小**：`115 bytes`
- **替换建议**：现在可以不动。如果以后你要恢复顶部栏文字 Logo，再替换它

## 2. Tauri 主程序 / 安装器图标

这些文件是打包时真正会用到的主图标资源。

### 2.1 主入口图标

- **文件**：`src-tauri/icons/icon.ico`
- **用途**：Windows 主程序图标、安装器图标的核心来源
- **配置引用**：
  - `src-tauri/tauri.conf.json`
  - `src-tauri/tauri.windows.conf.json`
  - `src-tauri/webview2.x64.json`
  - `src-tauri/webview2.x86.json`
  - `src-tauri/webview2.arm64.json`
- **当前内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
- **当前文件大小**：`76031 bytes`
- **替换建议**：如果你只换一个 Windows 主图标，最关键就是这个文件

### 2.2 主图标 PNG / ICNS 派生文件

- **文件**：`src-tauri/icons/32x32.png`
  - **尺寸**：`32 x 32`
  - **文件大小**：`2501 bytes`
  - **用途**：Tauri bundle icon 列表中的小尺寸 PNG

- **文件**：`src-tauri/icons/128x128.png`
  - **尺寸**：`128 x 128`
  - **文件大小**：`22235 bytes`
  - **用途**：Tauri bundle icon 列表中的标准 PNG

- **文件**：`src-tauri/icons/128x128@2x.png`
  - **尺寸**：`256 x 256`
  - **文件大小**：`58501 bytes`
  - **用途**：Tauri bundle icon 列表中的高分辨率 PNG

- **文件**：`src-tauri/icons/icon.png`
  - **尺寸**：`512 x 512`
  - **文件大小**：`161640 bytes`
  - **用途**：较大的主图标 PNG 源

- **文件**：`src-tauri/icons/icon.icns`
  - **类型**：`ICNS container`
  - **文件大小**：`868158 bytes`
  - **用途**：macOS 应用图标

### 2.3 Windows Store / Appx 相关图标

- **文件**：`src-tauri/icons/Square30x30Logo.png`
  - **尺寸**：`30 x 30`
  - **文件大小**：`2245 bytes`

- **文件**：`src-tauri/icons/Square44x44Logo.png`
  - **尺寸**：`44 x 44`
  - **文件大小**：`4263 bytes`

- **文件**：`src-tauri/icons/Square71x71Logo.png`
  - **尺寸**：`71 x 71`
  - **文件大小**：`9132 bytes`

- **文件**：`src-tauri/icons/Square89x89Logo.png`
  - **尺寸**：`89 x 89`
  - **文件大小**：`12914 bytes`

- **文件**：`src-tauri/icons/Square107x107Logo.png`
  - **尺寸**：`107 x 107`
  - **文件大小**：`17018 bytes`

- **文件**：`src-tauri/icons/Square142x142Logo.png`
  - **尺寸**：`142 x 142`
  - **文件大小**：`25624 bytes`

- **文件**：`src-tauri/icons/Square150x150Logo.png`
  - **尺寸**：`150 x 150`
  - **文件大小**：`27892 bytes`

- **文件**：`src-tauri/icons/Square284x284Logo.png`
  - **尺寸**：`284 x 284`
  - **文件大小**：`66264 bytes`

- **文件**：`src-tauri/icons/Square310x310Logo.png`
  - **尺寸**：`310 x 310`
  - **文件大小**：`74559 bytes`

- **文件**：`src-tauri/icons/StoreLogo.png`
  - **尺寸**：`50 x 50`
  - **文件大小**：`5233 bytes`

## 3. 托盘图标

### 3.1 当前 Windows 默认托盘图标

这 3 个是当前已经统一过的 Windows 默认托盘图标。

- **文件**：`src-tauri/icons/tray-icon.ico`
  - **用途**：普通状态托盘图标
  - **当前内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
  - **当前文件大小**：`76031 bytes`

- **文件**：`src-tauri/icons/tray-icon-sys.ico`
  - **用途**：系统代理状态托盘图标
  - **当前内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
  - **当前文件大小**：`76031 bytes`

- **文件**：`src-tauri/icons/tray-icon-tun.ico`
  - **用途**：TUN 状态托盘图标
  - **当前内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
  - **当前文件大小**：`76031 bytes`

### 3.2 macOS 单色托盘图标

这些图标现在**不建议随便改成彩色版本**，因为它们通常用于 macOS 的单色模板托盘逻辑。

- **文件**：`src-tauri/icons/tray-icon-mono.ico`
  - **内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
  - **文件大小**：`15215 bytes`

- **文件**：`src-tauri/icons/tray-icon-sys-mono.ico`
  - **内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
  - **文件大小**：`17598 bytes`

- **文件**：`src-tauri/icons/tray-icon-sys-mono-new.ico`
  - **内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
  - **文件大小**：`45233 bytes`

- **文件**：`src-tauri/icons/tray-icon-tun-mono.ico`
  - **内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
  - **文件大小**：`16069 bytes`

- **文件**：`src-tauri/icons/tray-icon-tun-mono-new.ico`
  - **内含尺寸**：`16x16, 24x24, 32x32, 48x48, 64x64, 256x256`
  - **文件大小**：`42037 bytes`

## 4. 你后面自己替换时，优先顺序建议

### 4.1 如果你只想先把界面看起来换掉

优先替换：

- `src/assets/image/icon_dark.svg`

### 4.2 如果你要把 Windows 桌面打包图标也一起换掉

优先替换：

- `src-tauri/icons/icon.ico`
- `src-tauri/icons/32x32.png`
- `src-tauri/icons/128x128.png`
- `src-tauri/icons/128x128@2x.png`
- `src-tauri/icons/icon.png`

### 4.3 如果你要连托盘图标一起统一

再替换：

- `src-tauri/icons/tray-icon.ico`
- `src-tauri/icons/tray-icon-sys.ico`
- `src-tauri/icons/tray-icon-tun.ico`

### 4.4 如果你要兼顾 macOS 单色托盘风格

最后再单独设计并替换：

- `src-tauri/icons/tray-icon-mono.ico`
- `src-tauri/icons/tray-icon-sys-mono.ico`
- `src-tauri/icons/tray-icon-sys-mono-new.ico`
- `src-tauri/icons/tray-icon-tun-mono.ico`
- `src-tauri/icons/tray-icon-tun-mono-new.ico`

## 5. 素材准备建议

- **前端 SVG 图标**
  - 建议准备正方形画布
  - 当前基准：`48 x 48`
  - 实际显示：`22 x 22 px`

- **Windows ICO**
  - 建议至少包含：`16, 24, 32, 48, 64, 256`
  - 背景建议透明

- **PNG 主图标**
  - 建议从较大透明底源图导出
  - 常用基准：`512 x 512`

- **文字 Logo**
  - 如果以后要恢复顶部栏文字标识，可按：`156 x 24` 去设计
  - 当前这个文件可以继续保持空白，不影响现在运行

## 6. 当前最小替换集合

如果你想用最少文件完成大部分视觉统一，建议只准备下面这些：

- `src/assets/image/icon_dark.svg`
- `src-tauri/icons/icon.ico`
- `src-tauri/icons/icon.png`
- `src-tauri/icons/32x32.png`
- `src-tauri/icons/128x128.png`
- `src-tauri/icons/128x128@2x.png`
- `src-tauri/icons/tray-icon.ico`
- `src-tauri/icons/tray-icon-sys.ico`
- `src-tauri/icons/tray-icon-tun.ico`

这样基本就能覆盖：

- 前端顶部图标
- Windows 主程序图标
- 安装器图标
- Windows 默认托盘图标
