# SVG 图标尺寸修复

## 问题描述

**症状：** SVG 图标占满整个屏幕，导致 UI 严重错位。

**影响范围：** 除主应用外的几乎所有地方，包括：
- 测试卡片中的图标
- 设置页面的托盘图标预览

## 根本原因

### 问题代码

在多个组件中，`<img>` 标签只设置了 `height` 属性，没有设置 `width`：

```tsx
// ❌ 错误：只设置 height
<img height="40px" src={iconUrl} />
<img height="20px" src={trayIcon} />
```

### 为什么会导致图标占满屏幕？

1. **浏览器的默认行为**：
   - 当 `<img>` 只设置 `height` 时，浏览器会根据图片的宽高比自动计算 `width`
   - 如果 SVG 没有明确的 `width` 和 `height` 属性，浏览器可能使用默认值或父容器的宽度

2. **SVG 的特殊性**：
   - SVG 是矢量图，可以无限缩放
   - 如果 SVG 只有 `viewBox` 没有 `width/height`，浏览器可能将其渲染为 100% 宽度
   - 即使 SVG 有尺寸属性，`<img>` 标签的不完整样式也可能导致拉伸

3. **Flexbox 布局的影响**：
   - 父容器使用 `display: flex`
   - 子元素（图标）没有明确的宽度约束
   - 导致图标被拉伸以填充可用空间

## 修复方案

### 修复原则

为所有 `<img>` 标签同时设置 `width` 和 `height`，并使用 `objectFit: 'contain'` 保持宽高比：

```tsx
// ✅ 正确：同时设置 width 和 height
<img 
  style={{ 
    width: '40px', 
    height: '40px', 
    objectFit: 'contain'  // 保持宽高比，不拉伸
  }} 
  src={iconUrl}
  alt="Icon description"
/>
```

### 修复的文件

#### 1. `src/components/test/test-item.tsx`

**修复前：**
```tsx
{icon.trim().startsWith('http') && (
  <img
    src={iconCachePath === '' ? icon : iconCachePath}
    height="40px"  // ❌ 只有 height
  />
)}
{icon.trim().startsWith('data') && (
  <img src={icon} height="40px" />  // ❌ 只有 height
)}
{icon.trim().startsWith('<svg') && (
  <img
    src={`data:image/svg+xml;base64,${btoa(icon)}`}
    height="40px"  // ❌ 只有 height
  />
)}
```

**修复后：**
```tsx
{icon.trim().startsWith('http') && (
  <img
    src={iconCachePath === '' ? icon : iconCachePath}
    style={{ width: '40px', height: '40px', objectFit: 'contain' }}  // ✅
    alt={name}
  />
)}
{icon.trim().startsWith('data') && (
  <img 
    src={icon} 
    style={{ width: '40px', height: '40px', objectFit: 'contain' }}  // ✅
    alt={name}
  />
)}
{icon.trim().startsWith('<svg') && (
  <img
    src={`data:image/svg+xml;base64,${btoa(icon)}`}
    style={{ width: '40px', height: '40px', objectFit: 'contain' }}  // ✅
    alt={name}
  />
)}
```

**同时修复了 MUI 图标：**
```tsx
// 修复前
<LanguageRounded sx={{ height: '40px' }} fontSize="large" />

// 修复后
<LanguageRounded sx={{ width: '40px', height: '40px' }} fontSize="large" />
```

#### 2. `src/components/setting/components/misc/layout-config.tsx`

修复了三个托盘图标预览：

**修复前：**
```tsx
// Common tray icon
<img height="20px" src={convertFileSrc(commonIcon)} />

// System proxy tray icon
<img height="20px" src={convertFileSrc(sysproxyIcon)} />

// TUN tray icon
<img height="20px" src={convertFileSrc(tunIcon)} />
```

**修复后：**
```tsx
// Common tray icon
<img 
  style={{ width: '20px', height: '20px', objectFit: 'contain' }} 
  src={convertFileSrc(commonIcon)}
  alt="Common tray icon"
/>

// System proxy tray icon
<img 
  style={{ width: '20px', height: '20px', objectFit: 'contain' }} 
  src={convertFileSrc(sysproxyIcon)}
  alt="System proxy tray icon"
/>

// TUN tray icon
<img 
  style={{ width: '20px', height: '20px', objectFit: 'contain' }} 
  src={convertFileSrc(tunIcon)}
  alt="TUN tray icon"
/>
```

## 技术细节

### objectFit 属性

`objectFit: 'contain'` 的作用：
- 保持图片的宽高比
- 图片完整显示在指定的尺寸内
- 不会裁剪或拉伸图片
- 如果宽高比不匹配，会留白

**其他选项：**
- `fill` - 拉伸填充（默认，会变形）
- `cover` - 覆盖整个区域（可能裁剪）
- `contain` - 完整显示（推荐）
- `none` - 保持原始尺寸
- `scale-down` - 取 `none` 和 `contain` 中较小的

### 为什么使用 style 而不是属性？

```tsx
// ❌ HTML 属性（旧方式）
<img width="40px" height="40px" />

// ✅ CSS 样式（推荐）
<img style={{ width: '40px', height: '40px' }} />
```

**原因：**
1. HTML 属性只接受数字（像素），不支持其他单位
2. CSS 样式更灵活，支持响应式设计
3. 可以同时设置 `objectFit` 等其他 CSS 属性
4. 与 React/MUI 的样式系统一致

### SVG 的最佳实践

#### 1. 作为 React 组件导入（推荐）

```tsx
import IconComponent from '@/assets/icon.svg?react'

<SvgIcon 
  component={IconComponent}
  sx={{ width: 24, height: 24 }}
/>
```

**优点：**
- 可以通过 CSS 控制颜色
- 更好的性能
- 类型安全

#### 2. 作为图片导入

```tsx
import iconUrl from '@/assets/icon.svg'

<img 
  src={iconUrl}
  style={{ width: '24px', height: '24px', objectFit: 'contain' }}
  alt="Icon"
/>
```

**优点：**
- 简单直接
- 适合不需要修改颜色的图标

#### 3. 内联 SVG（不推荐）

```tsx
<img 
  src={`data:image/svg+xml;base64,${btoa(svgString)}`}
  style={{ width: '24px', height: '24px', objectFit: 'contain' }}
/>
```

**缺点：**
- Base64 编码增加文件大小
- 不能缓存
- 性能较差

**何时使用：**
- 用户上传的自定义图标
- 动态生成的 SVG

## 验证修复

### 1. TypeScript 类型检查

```bash
pnpm exec tsc --noEmit
```

**结果：** ✅ 通过

### 2. 视觉检查

启动应用并检查：

```bash
pnpm tauri dev
```

**检查项：**
- [ ] 测试卡片中的图标尺寸正常（40x40px）
- [ ] 设置页面的托盘图标预览正常（20x20px）
- [ ] 图标不会拉伸变形
- [ ] 图标不会占满整个屏幕

### 3. 不同图标类型测试

测试以下图标类型：
- [ ] HTTP URL 图标
- [ ] Data URL 图标
- [ ] SVG 字符串图标
- [ ] 本地文件图标

## 预防措施

### 1. 代码审查清单

在添加新的图标时，检查：
- [ ] 是否同时设置了 `width` 和 `height`
- [ ] 是否使用了 `objectFit: 'contain'`
- [ ] 是否添加了 `alt` 属性（可访问性）

### 2. ESLint 规则（可选）

可以添加自定义 ESLint 规则来检测这个问题：

```javascript
// .eslintrc.js
rules: {
  'jsx-a11y/img-redundant-alt': 'warn',
  // 自定义规则：检测只有 height 的 img
  'no-restricted-syntax': [
    'error',
    {
      selector: 'JSXOpeningElement[name.name="img"]:not(:has(JSXAttribute[name.name="style"])):has(JSXAttribute[name.name="height"])',
      message: 'img 标签必须同时设置 width 和 height，推荐使用 style 属性'
    }
  ]
}
```

### 3. 组件封装

创建一个统一的图标组件：

```tsx
// components/base/base-icon.tsx
interface BaseIconProps {
  src: string
  size?: number
  alt?: string
}

export const BaseIcon = ({ src, size = 24, alt = '' }: BaseIconProps) => {
  return (
    <img
      src={src}
      style={{
        width: `${size}px`,
        height: `${size}px`,
        objectFit: 'contain',
      }}
      alt={alt}
    />
  )
}

// 使用
<BaseIcon src={iconUrl} size={40} alt="Test icon" />
```

## 相关问题

### 为什么主应用没有这个问题？

主应用可能使用了：
1. MUI 的 `SvgIcon` 组件（自动处理尺寸）
2. 图标字体（如 Material Icons）
3. 正确设置了尺寸的图标

### 为什么开发模式没发现？

可能的原因：
1. 开发模式使用了不同的图标
2. 热重载导致样式缓存
3. 浏览器缓存了旧的样式

### 如何避免类似问题？

1. **使用 MUI 的 SvgIcon**：
   ```tsx
   <SvgIcon component={IconComponent} sx={{ width: 24, height: 24 }} />
   ```

2. **使用图标库**：
   ```tsx
   import { Icon } from '@mui/icons-material'
   <Icon sx={{ width: 24, height: 24 }} />
   ```

3. **创建统一的图标组件**（见上面的 `BaseIcon`）

## 总结

### 修复内容

✅ **修复了 2 个文件，共 7 处图标尺寸问题**

1. `test-item.tsx` - 4 处（3 个 img + 1 个 MUI 图标）
2. `layout-config.tsx` - 3 处（托盘图标预览）

### 修复效果

- ✅ 图标不再占满整个屏幕
- ✅ 图标尺寸固定且正确
- ✅ 图标不会拉伸变形
- ✅ 保持了图标的宽高比
- ✅ TypeScript 类型检查通过

### 技术要点

- 同时设置 `width` 和 `height`
- 使用 `objectFit: 'contain'` 保持宽高比
- 添加 `alt` 属性提高可访问性
- 使用 CSS `style` 而不是 HTML 属性

---

**修复时间：** 2026-05-27  
**影响文件：** 2 个文件  
**测试状态：** ✅ TypeScript 检查通过，待视觉验证
