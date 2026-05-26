# 字体配置指南 (Font Configuration Guide)

## 📋 概述

本项目已将字体配置统一为**主题系统作为唯一数据源**，解决了之前 SCSS 和主题系统之间的配置冲突问题。

## ✅ 已完成的修改

### 1. 移除 SCSS 中的硬编码字体定义

**修改文件：**
- `src/assets/styles/index.scss`
- `src/assets/styles/layout.scss`

**变更内容：**
- 移除所有 `font-family: 'Outfit', 'Inter', ... !important` 定义
- 改用 CSS 变量 `var(--font-family)` 统一引用

### 2. 更新主题默认值

**修改文件：**
- `src/pages/_theme.tsx`

**变更内容：**
- 将 UDS 规范字体 `'Outfit', 'Inter'` 设置为默认字体优先级最高
- 完整字体栈：`'Outfit', 'Inter', -apple-system, BlinkMacSystemFont, ...`

### 3. 主题系统集成

**修改文件：**
- `src/pages/_layout/hooks/use-custom-theme.ts`

**变更内容：**
- 简化字体合并逻辑：`setting.font_family || dt.font_family`
- 添加 CSS 变量 `--font-family` 设置，作为全局字体的唯一数据源
- 所有 SCSS 通过 CSS 变量引用字体配置

## 🎯 配置链路

```
用户自定义字体 (theme-viewer.tsx)
    ↓
主题配置 (verge.theme_setting.font_family)
    ↓
主题 Hook (use-custom-theme.ts)
    ↓
CSS 变量 (--font-family)
    ↓
全局应用 (SCSS 通过 var(--font-family) 引用)
```

## 🔧 如何使用

### 用户自定义字体

1. 打开设置 → 主题设置
2. 修改"字体系列"字段
3. 保存后立即生效

**示例：**
```
Arial, sans-serif
```

### 开发者修改默认字体

编辑 `src/pages/_theme.tsx`：

```typescript
export const defaultTheme = {
  // ...
  font_family: `'Outfit', 'Inter', -apple-system, ...`,
}
```

## 📦 字体加载方式

### 当前方式：Google Fonts CDN

**位置：** `src/assets/styles/index.scss` 第78行

```scss
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;600;900&family=Inter:wght@400;600;900&display=swap');
```

**优点：**
- ✅ 无需下载，自动更新
- ✅ 减小打包体积
- ✅ 浏览器缓存优化

**缺点：**
- ⚠️ 需要网络连接
- ⚠️ 首次加载稍慢

### 可选方式：本地字体文件

如需离线使用，可切换到本地字体：

#### 1. 下载字体文件

**Outfit 字体：**
- 下载：https://fonts.google.com/specimen/Outfit
- 字重：400, 600, 900
- 格式：WOFF2（推荐）

**Inter 字体：**
- 下载：https://fonts.google.com/specimen/Inter
- 字重：400, 600, 900
- 格式：WOFF2（推荐）

#### 2. 存放位置

```
src/assets/fonts/
├── Outfit-Regular.woff2
├── Outfit-SemiBold.woff2
├── Outfit-Black.woff2
├── Inter-Regular.woff2
├── Inter-SemiBold.woff2
├── Inter-Black.woff2
└── Twemoji.Mozilla.ttf (已存在)
```

#### 3. 修改 font.scss

编辑 `src/assets/styles/font.scss`：

```scss
@font-face {
  font-family: 'Outfit';
  src: url('../fonts/Outfit-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Outfit';
  src: url('../fonts/Outfit-SemiBold.woff2') format('woff2');
  font-weight: 600;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Outfit';
  src: url('../fonts/Outfit-Black.woff2') format('woff2');
  font-weight: 900;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Inter';
  src: url('../fonts/Inter-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Inter';
  src: url('../fonts/Inter-SemiBold.woff2') format('woff2');
  font-weight: 600;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Inter';
  src: url('../fonts/Inter-Black.woff2') format('woff2');
  font-weight: 900;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'twemoji mozilla';
  src: url('../fonts/Twemoji.Mozilla.ttf');
}
```

#### 4. 移除 CDN 引用

从 `src/assets/styles/index.scss` 中删除：

```scss
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;600;900&family=Inter:wght@400;600;900&display=swap');
```

## 🧪 测试验证

### 1. 类型检查
```bash
pnpm run typecheck
```

### 2. 构建测试
```bash
pnpm run build
```

### 3. 运行测试
```bash
pnpm run dev
```

### 4. 功能验证
- [ ] 打开应用，检查字体是否正确显示
- [ ] 进入设置 → 主题设置
- [ ] 修改字体系列为 `Arial, sans-serif`
- [ ] 保存后检查全局字体是否变更
- [ ] 恢复默认字体，检查是否回到 Outfit/Inter

## 📊 优势对比

| 项目 | 修改前 | 修改后 |
|------|--------|--------|
| 配置位置 | SCSS + 主题系统（冲突） | 主题系统（唯一） |
| 用户自定义 | ❌ 无效（被 !important 覆盖） | ✅ 有效 |
| 维护难度 | 🔴 高（多处定义） | 🟢 低（单一数据源） |
| 配置优先级 | 混乱 | 清晰 |
| 代码可读性 | 🔴 差 | 🟢 好 |

## 🔍 技术细节

### CSS 变量作用域

`--font-family` 变量设置在 `:root` (document.documentElement)，全局可用：

```typescript
rootEle.style.setProperty('--font-family', resolvedFontFamily)
```

### SCSS 引用方式

所有需要字体的地方统一使用：

```scss
body, html {
  font-family: var(--font-family);
}

.some-element {
  font-family: var(--font-family);
}
```

### 回退机制

如果用户未设置自定义字体，自动使用默认值：

```typescript
const resolvedFontFamily = setting.font_family || dt.font_family
```

## 📝 注意事项

1. **不要在 SCSS 中硬编码字体**：始终使用 `var(--font-family)`
2. **不要使用 !important**：会破坏主题系统的优先级
3. **保持字体栈完整**：确保有足够的回退字体
4. **测试多平台**：Windows/macOS/Linux 字体渲染可能不同

## 🎉 总结

通过本次重构，我们实现了：
- ✅ 消除配置冲突
- ✅ 统一数据源
- ✅ 用户可自定义字体
- ✅ 代码更易维护
- ✅ 符合 UDS 规范

字体配置现在完全由主题系统管理，用户可以通过 UI 自由定制，开发者维护更加简单！
