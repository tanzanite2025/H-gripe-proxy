# UI 错位问题深度诊断清单

## 问题描述

生产构建后，UI 出现错位、样式丢失或布局混乱的问题。开发环境正常，但生产环境异常。

## 诊断清单

### ✅ 1. CSP（内容安全策略）配置

**检查项：**
- [x] `style-src` 包含 `'self' 'unsafe-inline' https://fonts.googleapis.com`
- [x] `font-src` 包含 `'self' data: https://fonts.gstatic.com`
- [x] `script-src` 包含 `'self' 'unsafe-inline' 'unsafe-eval'`

**当前配置：** ✅ 正确

```json
"csp": "default-src 'self'; connect-src 'self' 127.0.0.1 http://127.0.0.1:* ws://127.0.0.1:* https://* http://*; img-src 'self' asset: data: https://*; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' data: https://fonts.gstatic.com; script-src 'self' 'unsafe-inline' 'unsafe-eval';"
```

### 🔍 2. CSS 文件生成

**检查项：**
```powershell
Get-ChildItem dist\assets\*.css | Select-Object Name, Length
```

**预期结果：**
- `index-*.css` - 主样式文件（~18KB）
- `editor-*.css` - Monaco Editor 样式（~146KB）

**验证：**
```powershell
# 检查 CSS 文件内容
Get-Content dist\assets\index-*.css -Head 20
```

应该包含项目的样式规则（如 `.uds-*`, `.layout`, 等）。

### 🔍 3. HTML 资源引用

**检查项：**
```powershell
Get-Content dist\index.html
```

**必须包含：**
- `<link rel="stylesheet" ... href="/assets/index-*.css">`
- `<script ... src="/assets/index-*.js"></script>`

**验证路径：**
- 路径必须以 `/` 开头（绝对路径）
- 不能是相对路径（`./` 或 `../`）

### 🔍 4. Vite 配置

**检查项：** `vite.config.mts`

```typescript
export default defineConfig({
  root: 'src',  // ← 重要！
  build: {
    outDir: '../dist',  // ← 相对于 root
    emptyOutDir: true,
  },
  resolve: {
    alias: {
      '@': path.resolve('./src'),  // ← 路径别名
    },
  },
})
```

**常见问题：**
- `root` 设置错误导致路径解析问题
- `alias` 配置不正确导致导入失败

### 🔍 5. 样式导入顺序

**检查项：** `src/main.tsx`

```typescript
import './assets/styles/index.scss'  // ← 必须在最前面
```

**验证：**
- 样式导入必须在所有组件导入之前
- 确保路径正确

### 🔍 6. SCSS 编译

**检查项：**
```powershell
# 检查是否有 SCSS 编译错误
pnpm run web:build 2>&1 | Select-String -Pattern "error|warning"
```

**常见问题：**
- SCSS 语法错误
- `@use` 或 `@import` 路径错误
- 变量未定义

### 🔍 7. Emotion/MUI 样式注入

**检查项：** `src/index.html`

```html
<meta name="emotion-insertion-point" content="" />
```

**作用：**
- 控制 Emotion（MUI 使用的 CSS-in-JS）样式注入位置
- 确保 Emotion 样式不会覆盖全局样式

**验证：**
- 这个 meta 标签必须在 `<head>` 中
- 必须在其他样式标签之前

### 🔍 8. 字体加载

**检查项：** `src/assets/styles/index.scss`

```scss
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;600;900&family=Inter:wght@400;600;900&display=swap');
```

**验证：**
```powershell
# 在浏览器开发工具中检查
# Network 标签 → 查找 fonts.googleapis.com 请求
# 应该返回 200 状态码
```

**常见问题：**
- CSP 阻止字体加载
- 网络问题导致字体加载失败
- 字体 URL 错误

### 🔍 9. CSS 变量定义

**检查项：** `src/assets/styles/index.scss`

```scss
:root {
  --primary-main: #5b5c9d;
  --text-primary: #1f1f1f;
  --background-color: #f5f5f5;
  --card-bg: #ffffff;
  --font-family: 'Outfit', 'Inter', ...;
  // ... 等等
}
```

**验证：**
```powershell
# 检查生成的 CSS 是否包含这些变量
Get-Content dist\assets\index-*.css | Select-String -Pattern "--primary-main|--font-family"
```

### 🔍 10. 浏览器控制台错误

**检查项：**
1. 打开应用
2. 按 `F12` 打开开发者工具
3. 查看 Console 标签

**常见错误：**
- `Refused to load ...` - CSP 错误
- `Failed to load resource` - 资源路径错误
- `Uncaught SyntaxError` - JavaScript 错误
- `Uncaught TypeError` - 运行时错误

### 🔍 11. Network 请求

**检查项：**
1. 开发者工具 → Network 标签
2. 刷新页面
3. 检查所有请求状态

**必须成功加载：**
- `index.html` - 200
- `index-*.css` - 200
- `index-*.js` - 200
- `fonts.googleapis.com` - 200
- `fonts.gstatic.com` - 200

**常见问题：**
- 404 - 文件路径错误
- 403 - CSP 阻止
- ERR_BLOCKED_BY_CLIENT - 广告拦截器

### 🔍 12. 样式优先级冲突

**检查项：**
1. 开发者工具 → Elements 标签
2. 选择一个错位的元素
3. 查看 Styles 面板

**检查：**
- 样式是否被覆盖（有删除线）
- 是否有 `!important` 冲突
- 是否有内联样式覆盖

### 🔍 13. Tauri WebView 特定问题

**检查项：**

**Windows WebView2：**
```powershell
# 检查 WebView2 版本
Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" | Select-Object pv
```

**常见问题：**
- WebView2 版本过旧
- WebView2 未安装
- WebView2 缓存问题

**清理 WebView2 缓存：**
```powershell
Remove-Item "$env:LOCALAPPDATA\Clash Verge Optimized\EBWebView" -Recurse -Force
```

### 🔍 14. 构建模式差异

**检查项：**

**开发模式：**
```bash
pnpm tauri dev
```
- 使用 Vite 开发服务器
- 热重载
- 未压缩的代码
- Source maps

**生产模式：**
```bash
pnpm build
```
- 静态文件
- 代码压缩
- 无 source maps（默认）
- 严格的 CSP

**验证：**
```powershell
# 检查是否有 source map
Get-ChildItem dist\assets\*.map
```

如果有 `.map` 文件，说明 source maps 已启用（有助于调试）。

### 🔍 15. 主题模式

**检查项：**

```typescript
// src/services/preload.ts
export const resolveThemeMode = (config: any): 'light' | 'dark' => {
  // ...
}
```

**验证：**
1. 检查应用是否正确检测系统主题
2. 检查 `[data-theme="dark"]` 样式是否生效
3. 检查 CSS 变量是否根据主题切换

**测试：**
```scss
// 深色模式样式
[data-theme='dark'] {
  :root {
    --background-color: #2e303d;
    --card-bg: #1a1b26;
    // ...
  }
}
```

## 诊断步骤

### 步骤 1：基础检查

```powershell
# 1. 清理并重新构建
.\clean-build.ps1
pnpm run web:build

# 2. 检查构建输出
Get-ChildItem dist\assets\*.css
Get-ChildItem dist\assets\*.js | Select-Object -First 5

# 3. 检查 HTML
Get-Content dist\index.html
```

### 步骤 2：运行应用并检查

```bash
# 运行开发模式
pnpm tauri dev
```

1. 按 `F12` 打开开发者工具
2. 检查 Console 是否有错误
3. 检查 Network 是否所有资源都加载成功
4. 检查 Elements → Styles 是否样式正确应用

### 步骤 3：对比开发和生产

**开发模式：**
```bash
pnpm tauri dev
```
- 记录 Console 输出
- 记录 Network 请求
- 截图正常的 UI

**生产模式：**
```bash
pnpm build
# 运行生成的安装包
```
- 记录 Console 输出
- 记录 Network 请求
- 截图错位的 UI

**对比：**
- 找出差异
- 定位问题来源

### 步骤 4：逐步排查

**如果 CSS 文件存在但样式不生效：**
1. 检查 CSP 配置
2. 检查 HTML 中的 `<link>` 标签
3. 检查浏览器控制台错误

**如果字体未加载：**
1. 检查 CSP 的 `font-src` 和 `style-src`
2. 检查 Network 标签中的字体请求
3. 尝试使用本地字体

**如果布局错位：**
1. 检查 CSS 变量是否定义
2. 检查是否有样式冲突
3. 检查 Flexbox/Grid 布局

**如果组件样式丢失：**
1. 检查 Emotion 样式注入
2. 检查 MUI 主题配置
3. 检查组件是否正确导入

## 常见问题和解决方案

### 问题 1：所有样式都丢失

**症状：** 页面显示为纯 HTML，无任何样式

**原因：** CSS 文件未加载或 CSP 阻止

**解决：**
1. 检查 `dist/assets/` 是否有 CSS 文件
2. 检查 `dist/index.html` 是否引用了 CSS
3. 检查 CSP 配置的 `style-src`
4. 检查浏览器控制台的 CSP 错误

### 问题 2：字体显示为系统默认字体

**症状：** 文字显示，但字体不对

**原因：** Google Fonts 未加载

**解决：**
1. 检查 CSP 的 `style-src` 和 `font-src`
2. 检查网络连接
3. 考虑使用本地字体

### 问题 3：部分组件样式丢失

**症状：** 大部分正常，但某些组件错位

**原因：** Emotion/MUI 样式注入问题

**解决：**
1. 检查 `<meta name="emotion-insertion-point">`
2. 检查 MUI 主题配置
3. 检查组件的 `sx` 或 `style` 属性

### 问题 4：深色模式不工作

**症状：** 切换主题无效果

**原因：** 主题切换逻辑或 CSS 变量问题

**解决：**
1. 检查 `[data-theme="dark"]` 样式
2. 检查主题切换代码
3. 检查 CSS 变量是否正确定义

### 问题 5：开发正常，生产错位

**症状：** `pnpm tauri dev` 正常，`pnpm build` 后错位

**原因：** 构建配置或 CSP 差异

**解决：**
1. 检查 Vite 构建配置
2. 检查 CSP 配置
3. 检查代码压缩是否破坏了样式

## 调试工具

### 1. 浏览器开发者工具

```
F12 → Console    # 查看错误
F12 → Network    # 查看资源加载
F12 → Elements   # 查看样式应用
F12 → Application # 查看缓存
```

### 2. Tauri 开发工具

```bash
# 启用开发工具
pnpm tauri dev

# 在应用中按 F12
```

### 3. 样式检查脚本

```powershell
# 检查 CSS 文件内容
$css = Get-Content dist\assets\index-*.css -Raw
$css -match "\.uds-" ? "✓ UDS styles found" : "✗ UDS styles missing"
$css -match "--primary-main" ? "✓ CSS variables found" : "✗ CSS variables missing"
$css -match "font-family" ? "✓ Font styles found" : "✗ Font styles missing"
```

### 4. 网络请求检查

```javascript
// 在浏览器控制台运行
performance.getEntriesByType('resource')
  .filter(r => r.name.includes('.css') || r.name.includes('font'))
  .forEach(r => console.log(r.name, r.transferSize))
```

## 最终检查清单

在报告问题前，请确认：

- [ ] 已清理构建缓存（`.\clean-build.ps1`）
- [ ] 已重新构建（`pnpm run web:build`）
- [ ] 已检查 CSP 配置
- [ ] 已检查浏览器控制台错误
- [ ] 已检查 Network 标签的资源加载
- [ ] 已对比开发和生产模式
- [ ] 已清理 WebView2 缓存
- [ ] 已尝试在不同的 Windows 版本/机器上测试

## 获取帮助

如果以上步骤都无法解决问题，请提供：

1. **浏览器控制台截图**（Console 和 Network 标签）
2. **UI 错位截图**（标注哪里不对）
3. **构建日志**（`pnpm run web:build` 的完整输出）
4. **系统信息**：
   - Windows 版本
   - WebView2 版本
   - Node.js 版本
   - pnpm 版本

---

**创建时间：** 2026-05-27  
**最后更新：** 2026-05-27  
**适用版本：** 0.0.3+
