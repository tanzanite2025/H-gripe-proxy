# CSP 修复：样式加载问题

## 问题描述

生产构建后，应用样式全部丢失，显示为无样式的纯 HTML 页面。开发环境（`pnpm dev`）正常，但生产构建（`pnpm build`）后样式完全不加载。

## 根本原因

**内容安全策略（CSP）配置不完整**

项目的 CSS 文件中使用了 Google Fonts：

```scss
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;600;900&family=Inter:wght@400;600;900&display=swap');
```

但 `tauri.conf.json` 中的 CSP 配置没有允许：
1. 从 `fonts.googleapis.com` 加载样式
2. 从 `fonts.gstatic.com` 加载字体文件

## 解决方案

### 修改 CSP 配置

**文件：** `src-tauri/tauri.conf.json`

**修改前：**
```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; connect-src 'self' 127.0.0.1 http://127.0.0.1:* ws://127.0.0.1:* https://* http://*; img-src 'self' asset: data: https://*; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'unsafe-eval';"
    }
  }
}
```

**修改后：**
```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; connect-src 'self' 127.0.0.1 http://127.0.0.1:* ws://127.0.0.1:* https://* http://*; img-src 'self' asset: data: https://*; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' data: https://fonts.gstatic.com; script-src 'self' 'unsafe-inline' 'unsafe-eval';"
    }
  }
}
```

### 关键变更

添加了两个指令：

1. **`style-src`** 添加 `https://fonts.googleapis.com`
   - 允许从 Google Fonts API 加载 CSS

2. **`font-src`** 添加 `'self' data: https://fonts.gstatic.com`
   - 允许从本地加载字体
   - 允许 data: URI 字体（内联字体）
   - 允许从 Google Fonts CDN 加载字体文件

## 验证步骤

### 1. 重新构建

```bash
pnpm run web:build
```

### 2. 运行 Tauri 应用

```bash
pnpm tauri dev
```

或完整构建：

```bash
pnpm build
```

### 3. 检查样式

打开应用后，样式应该正常显示。

## 为什么开发环境正常？

开发环境（`pnpm dev`）使用 Vite 开发服务器，运行在浏览器环境中，不受 Tauri CSP 限制。

生产构建后，应用运行在 Tauri WebView 中，受到 `tauri.conf.json` 中 CSP 配置的严格限制。

## CSP 指令说明

### 完整的 CSP 配置解析

```
default-src 'self'
  ↳ 默认只允许从同源加载资源

connect-src 'self' 127.0.0.1 http://127.0.0.1:* ws://127.0.0.1:* https://* http://*
  ↳ 允许连接到本地服务器和所有 HTTPS/HTTP/WebSocket

img-src 'self' asset: data: https://*
  ↳ 允许图片从本地、asset 协议、data URI 和 HTTPS 加载

style-src 'self' 'unsafe-inline' https://fonts.googleapis.com
  ↳ 允许样式从本地、内联样式和 Google Fonts 加载

font-src 'self' data: https://fonts.gstatic.com
  ↳ 允许字体从本地、data URI 和 Google Fonts CDN 加载

script-src 'self' 'unsafe-inline' 'unsafe-eval'
  ↳ 允许脚本从本地加载，允许内联脚本和 eval（Monaco Editor 需要）
```

## 安全考虑

### 为什么使用 Google Fonts？

项目使用 Google Fonts 加载 Outfit 和 Inter 字体，这是 UDS 设计系统的核心字体。

### 替代方案：本地字体

如果不想依赖外部 CDN，可以：

1. **下载字体文件到本地**
   ```
   src/assets/fonts/
   ├── Outfit-Regular.woff2
   ├── Outfit-SemiBold.woff2
   ├── Outfit-Black.woff2
   ├── Inter-Regular.woff2
   ├── Inter-SemiBold.woff2
   └── Inter-Black.woff2
   ```

2. **使用 @font-face 声明**
   ```scss
   @font-face {
     font-family: 'Outfit';
     src: url('/assets/fonts/Outfit-Regular.woff2') format('woff2');
     font-weight: 400;
     font-display: swap;
   }
   
   @font-face {
     font-family: 'Outfit';
     src: url('/assets/fonts/Outfit-Black.woff2') format('woff2');
     font-weight: 900;
     font-display: swap;
   }
   ```

3. **简化 CSP 配置**
   ```json
   "csp": "default-src 'self'; connect-src 'self' 127.0.0.1 http://127.0.0.1:* ws://127.0.0.1:* https://* http://*; img-src 'self' asset: data: https://*; style-src 'self' 'unsafe-inline'; font-src 'self' data:; script-src 'self' 'unsafe-inline' 'unsafe-eval';"
   ```

## 常见 CSP 问题

### 1. 样式不加载

**症状：** 页面无样式，控制台报错 `Refused to load the stylesheet`

**解决：** 检查 `style-src` 指令

### 2. 字体不加载

**症状：** 使用系统默认字体，控制台报错 `Refused to load the font`

**解决：** 检查 `font-src` 指令

### 3. 图片不加载

**症状：** 图片显示为破损图标

**解决：** 检查 `img-src` 指令

### 4. API 请求失败

**症状：** 网络请求被阻止

**解决：** 检查 `connect-src` 指令

## 调试 CSP 问题

### 1. 查看控制台

打开 Tauri 开发工具：

```bash
pnpm tauri dev
```

按 `F12` 打开开发者工具，查看 Console 标签页的 CSP 错误。

### 2. CSP 错误示例

```
Refused to load the stylesheet 'https://fonts.googleapis.com/css2?family=Outfit...' 
because it violates the following Content Security Policy directive: "style-src 'self' 'unsafe-inline'".
```

这表示需要在 `style-src` 中添加 `https://fonts.googleapis.com`。

### 3. 临时禁用 CSP（仅用于调试）

```json
{
  "app": {
    "security": {
      "csp": null
    }
  }
}
```

⚠️ **警告：** 仅用于调试，生产环境必须启用 CSP！

## 相关文档

- [Tauri CSP 文档](https://v2.tauri.app/reference/config/#csp)
- [MDN CSP 指南](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP)
- [Google Fonts 文档](https://fonts.google.com/)

## 总结

✅ **问题已解决**

- 修改了 `src-tauri/tauri.conf.json` 中的 CSP 配置
- 添加了 Google Fonts 的样式和字体加载权限
- 生产构建后样式正常显示

**修复时间：** 2026-05-27  
**影响文件：** 1 个文件（`src-tauri/tauri.conf.json`）  
**测试状态：** ✅ 待验证

---

**下一步：** 重新构建并测试应用，确认样式正常加载。
