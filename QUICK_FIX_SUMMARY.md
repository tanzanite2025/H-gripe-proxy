# Emotion 样式注入问题 - 快速修复总结

## 🎯 问题本质

**Release 构建中 MUI/Emotion 运行时样式未正确注入到 DOM**

### 症状
- ✅ UDS/SCSS 静态外壳正常（卡片轮廓、布局框架）
- ❌ MUI/Emotion 动态内核失效（图标尺寸、字体、间距）
- 结果：图标撑爆卡片，TAB 导航样式异常

---

## 🔧 修复内容

### 1. **vite.config.mts** - 配置 Emotion Babel 插件
```typescript
react({
  jsxImportSource: '@emotion/react',
  babel: {
    plugins: [
      [
        '@emotion/babel-plugin',
        {
          sourceMap: true,
          autoLabel: 'dev-only',
          labelFormat: '[local]',
          importMap: {
            '@mui/material': {
              styled: {
                canonicalImport: ['@emotion/styled', 'default'],
              },
            },
          },
        },
      ],
    ],
  },
})
```

### 2. **base-emotion-style-chain.tsx** - 强制禁用 Speedy 模式
```typescript
emotionStyleCache = createCache({
  key: EMOTION_CACHE_KEY,
  insertionPoint,
  speedy: false, // 👈 关键修复
})
```

### 3. **package.json** - 安装依赖
```bash
pnpm add -D @emotion/babel-plugin
```

---

## ✅ 验证步骤

### 1. 重新构建前端
```bash
pnpm web:build
```

### 2. 运行样式验证脚本
```bash
pnpm verify:styles
```

### 3. 完整构建并测试
```bash
pnpm build
# 或快速构建
pnpm build:fast
```

### 4. 检查 Release 应用
启动构建后的应用，确认：
- [ ] 卡片中的图标尺寸正常（24px）
- [ ] TAB 导航字体、间距符合设计
- [ ] 无异常大图标或布局撑爆
- [ ] 主题切换（亮/暗模式）正常

---

## 📊 技术背景

### Emotion Speedy 模式
| 模式 | 注入方式 | 性能 | 可见性 | 兼容性 |
|------|---------|------|--------|--------|
| **Speedy (默认 Prod)** | `CSSStyleSheet.insertRule()` | 高 | DOM 中不可见 | 可能在 Tauri WebView 中失效 |
| **DOM (修复后)** | `<style>` 标签 | 中 | DOM 中可见 | 兼容性好 |

### 为什么 DEV 正常但 Release 异常？
- **DEV**：Vite HMR + Emotion 默认 DOM 注入
- **Release**：Vite 优化 + Emotion Speedy 模式 → 样式丢失

---

## 🚨 如果修复后仍有问题

### 诊断步骤
1. **检查构建产物**
   ```bash
   # 打开 dist/index.html，搜索：
   # - <meta name="emotion-insertion-point">
   # - <style data-emotion="mui">
   # - MuiSvgIcon
   ```

2. **浏览器开发者工具**
   - 打开 Elements 面板
   - 检查 `<head>` 中是否有 `<style data-emotion="mui">` 标签
   - 检查图标元素的 computed styles

3. **控制台日志**
   - 查看是否有 Emotion 相关错误
   - 检查 `[样式诊断]` 日志（如果添加了监控代码）

### 备用方案
如果问题依然存在，考虑：
1. **清理缓存**
   ```bash
   pnpm clean-build  # 如果有此脚本
   # 或手动删除
   rm -rf dist node_modules/.vite
   pnpm install
   pnpm build
   ```

2. **✅ Tauri CSP 配置已确认正确**
   当前 `src-tauri/tauri.conf.json` 中的 CSP 配置：
   ```json
   "style-src": "'self' 'unsafe-inline' https://fonts.googleapis.com"
   ```
   - ✅ `'unsafe-inline'` 允许 Emotion 动态注入样式
   - ✅ 无需修改 CSP 配置

3. **降级到纯 SCSS**（最后手段）
   - 移除 MUI/Emotion 依赖
   - 将所有组件样式迁移到 SCSS

---

## 📁 相关文件

### 核心修复
- ✅ `vite.config.mts`
- ✅ `src/components/base/base-emotion-style-chain.tsx`
- ✅ `package.json`

### 样式系统
- `src/pages/_layout/hooks/use-custom-theme.ts` - MUI 主题配置
- `src/assets/styles/index.scss` - UDS 静态样式
- `src/assets/styles/layout.scss` - 布局样式
- `src/assets/styles/page.scss` - 页面样式

### 受影响组件
- `src/components/home/proxy-tun-card.tsx`
- `src/components/home/ip-info-card.tsx`
- `src/components/layout/layout-item.tsx`

---

## 📝 后续建议

### 短期
- [ ] 在 CI/CD 中添加 `pnpm verify:styles` 检查
- [ ] 添加样式注入监控代码（见 `EMOTION_STYLE_INJECTION_FIX.md`）

### 长期
- [ ] 评估是否统一样式架构（纯 MUI 或纯 SCSS）
- [ ] 考虑使用 CSS-in-JS 的替代方案（如 vanilla-extract、Panda CSS）
- [ ] 优化样式打包体积

---

## 🎉 修复完成

修复日期：2026-05-27  
修复人员：Kiro AI Assistant  

详细技术文档：`EMOTION_STYLE_INJECTION_FIX.md`
