# Emotion 样式注入修复方案

## 问题诊断

### 症状描述
Release 构建版本出现样式异常：
- **卡片被撑爆**：MUI 图标（如 `TroubleshootRounded`、`VisibilityOutlined`）尺寸异常大
- **TAB 导航异常**：字体、行高、间距不符合设计规范
- **双层样式分裂**：UDS/SCSS 静态外壳正常，但 MUI/Emotion 运行时内核失效

### 根本原因
项目采用**双层样式架构**：
1. **UDS/SCSS 静态层**：`src/assets/styles/*.scss` 提供基础外壳
2. **MUI/Emotion 动态层**：`use-custom-theme.ts` + `base-emotion-style-chain.tsx` 提供运行时样式

在 **production 构建**时，Vite 的 `@vitejs/plugin-react` 默认配置未正确处理 Emotion JSX pragma，导致：
- Emotion 的 `speedy` 模式在 release 下可能被错误启用
- 样式注入点（insertion point）未正确初始化
- MUI 组件的运行时样式（`MuiSvgIcon`、`MuiButton` 等）未完整注入到 DOM

结果：
- SCSS 外壳还在 → 卡片轮廓、布局框架正常
- Emotion 内核失效 → MUI 图标退回浏览器原生 `<svg>` 行为，尺寸失控

---

## 修复方案

### 1. 配置 Vite React 插件支持 Emotion

**文件**：`vite.config.mts`

```typescript
react({
  jsxImportSource: '@emotion/react',
  babel: {
    plugins: [
      [
        '@emotion/babel-plugin',
        {
          // 确保在 production 构建时也注入样式
          sourceMap: true,
          autoLabel: 'dev-only',
          labelFormat: '[local]',
          // 关键：强制使用 DOM 样式注入而非 speedy 模式
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

**关键点**：
- `jsxImportSource: '@emotion/react'`：告诉 Babel 使用 Emotion 的 JSX 运行时
- `@emotion/babel-plugin`：确保样式在编译时正确转换
- `importMap`：处理 MUI 组件的 styled 导入

### 2. 强化 Emotion 样式缓存配置

**文件**：`src/components/base/base-emotion-style-chain.tsx`

```typescript
emotionStyleCache = createCache({
  key: EMOTION_CACHE_KEY,
  insertionPoint,
  // 强制禁用 speedy 模式，确保样式始终注入到 DOM
  speedy: false,
})
```

**关键点**：
- `speedy: false`：强制禁用 Emotion 的 speedy 模式（该模式在 production 下会跳过 DOM 注入，直接操作 CSSOM，可能导致样式丢失）
- 双重保险：通过 `sheet.speedy(false)` API 再次确认

### 3. 安装必要依赖

```bash
pnpm add -D @emotion/babel-plugin
```

---

## 技术背景

### Emotion Speedy 模式
Emotion 在 production 模式下默认启用 `speedy` 模式：
- **优点**：通过 `CSSStyleSheet.insertRule()` 直接操作 CSSOM，性能更高
- **缺点**：样式不会出现在 DOM 的 `<style>` 标签中，某些场景下（如 SSR、Tauri WebView）可能导致样式丢失

### 为什么 DEV 正常但 Release 异常？
- **DEV 模式**：Vite HMR + Emotion 默认使用 DOM 注入，样式可见且可调试
- **Release 模式**：
  - Vite 生产构建优化可能触发 speedy 模式
  - 如果 Babel 插件未正确配置，Emotion JSX pragma 不会被转换
  - 结果：MUI 组件的运行时样式未注入，退回浏览器默认样式

---

## 验证步骤

### 1. 重新构建
```bash
pnpm build
```

### 2. 检查构建产物
打开 `dist/index.html`，检查：
- `<head>` 中是否存在 `<meta name="emotion-insertion-point">`
- 是否有 `<style data-emotion="mui">` 标签
- MUI 组件样式是否完整（搜索 `MuiSvgIcon`、`MuiButton` 等）

### 3. 运行 Release 版本
```bash
pnpm tauri build
```
启动应用，检查：
- 卡片中的图标尺寸是否正常（应为 24px）
- TAB 导航字体、间距是否符合设计
- 无异常大图标或布局撑爆

---

## 相关文件

### 样式系统核心文件
- `src/components/base/base-emotion-style-chain.tsx` - Emotion 缓存与注入点
- `src/pages/_layout/hooks/use-custom-theme.ts` - MUI 主题配置
- `src/assets/styles/index.scss` - UDS 静态样式入口
- `vite.config.mts` - Vite 构建配置

### 受影响的组件
- `src/components/home/proxy-tun-card.tsx` - 使用 `TroubleshootRounded` 图标
- `src/components/home/ip-info-card.tsx` - 使用 `VisibilityOutlined` 图标
- `src/components/layout/layout-item.tsx` - TAB 导航项
- 所有使用 MUI 组件的页面

---

## 后续优化建议

### 1. 考虑统一样式架构
当前双层架构（UDS + MUI）增加了维护复杂度，建议：
- **方案 A**：完全迁移到 MUI + Emotion（移除 SCSS）
- **方案 B**：完全使用 CSS Modules + SCSS（移除 MUI）
- **方案 C**：保持现状，但明确分层职责（SCSS 仅负责布局，MUI 负责组件）

### 2. 添加样式注入监控
在 `main.tsx` 中添加诊断代码：
```typescript
if (import.meta.env.PROD) {
  setTimeout(() => {
    const emotionStyles = document.querySelectorAll('style[data-emotion]')
    if (emotionStyles.length === 0) {
      console.error('[样式诊断] Emotion 样式未注入！')
    } else {
      console.log(`[样式诊断] 检测到 ${emotionStyles.length} 个 Emotion 样式标签`)
    }
  }, 1000)
}
```

### 3. 性能优化
如果确认样式注入稳定后，可以考虑：
- 在特定平台（如 Windows）重新启用 speedy 模式
- 使用 CSS 提取插件减少运行时开销

---

## 修复日期
2026-05-27

## 修复人员
Kiro AI Assistant

## 相关 Issue
- 卡片图标撑爆问题
- TAB 导航样式异常
- Release 构建样式丢失
