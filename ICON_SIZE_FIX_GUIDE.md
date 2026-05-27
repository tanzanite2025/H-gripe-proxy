# 🎨 图标尺寸修复指南

## 问题描述

打包后的应用中出现**超大图标**，导致布局混乱。这是因为 MUI 图标组件没有设置 `fontSize` 属性，使用了默认的超大尺寸。

## 根本原因

MUI v9 的图标默认尺寸变大了，需要显式设置 `fontSize` 属性。

## 修复方案

### 方案 1: 全局设置默认图标尺寸（推荐）

在主题配置中设置默认图标尺寸：

```typescript
// src/theme/index.ts
import { createTheme } from '@mui/material/styles'

export const theme = createTheme({
  // ... 其他配置
  
  components: {
    // 设置所有 MUI 图标的默认尺寸
    MuiSvgIcon: {
      defaultProps: {
        fontSize: 'medium', // 或 'small', 'large', 'inherit'
      },
      styleOverrides: {
        root: {
          // 确保图标尺寸合理
          fontSize: '1.5rem', // 24px
        },
        fontSizeSmall: {
          fontSize: '1.25rem', // 20px
        },
        fontSizeLarge: {
          fontSize: '2.1875rem', // 35px
        },
      },
    },
  },
})
```

### 方案 2: 批量修复现有代码

#### 2.1 修复规则

所有 MUI 图标都应该设置 `fontSize` 属性：

```typescript
// ❌ 错误 - 没有 fontSize
<SecurityOutlined color="primary" />
<InfoOutlined />
<CheckCircleOutlined />

// ✅ 正确 - 有 fontSize
<SecurityOutlined color="primary" fontSize="medium" />
<InfoOutlined fontSize="small" />
<CheckCircleOutlined fontSize="inherit" />
```

#### 2.2 fontSize 选项

| 值 | 尺寸 | 使用场景 |
|---|------|---------|
| `small` | 20px | 小图标、按钮图标 |
| `medium` | 24px | 默认尺寸、标题图标 |
| `large` | 35px | 大图标、强调图标 |
| `inherit` | 继承父元素 | IconButton 中的图标 |

#### 2.3 常见场景

**场景 1: IconButton 中的图标**
```typescript
// 使用 inherit 继承 IconButton 的尺寸
<IconButton>
  <RefreshOutlined fontSize="inherit" />
</IconButton>
```

**场景 2: 标题旁的图标**
```typescript
// 使用 medium 或 small
<Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
  <SecurityOutlined color="primary" fontSize="medium" />
  <Typography variant="h6">标题</Typography>
</Box>
```

**场景 3: Chip 中的图标**
```typescript
// 使用 small
<Chip
  label="标签"
  icon={<CheckCircleOutlined fontSize="small" />}
/>
```

**场景 4: Button startIcon/endIcon**
```typescript
// 使用 small 或 inherit
<Button
  startIcon={<AddOutlined fontSize="small" />}
>
  添加
</Button>
```

### 方案 3: 创建图标包装组件

创建一个带默认尺寸的图标包装组件：

```typescript
// src/components/ui/icon.tsx
import { SvgIconProps } from '@mui/material'

interface IconProps extends SvgIconProps {
  // 自定义属性
}

export function Icon({ fontSize = 'medium', ...props }: IconProps) {
  return <SvgIcon fontSize={fontSize} {...props} />
}

// 使用
import { SecurityOutlined } from '@mui/icons-material'
<Icon component={SecurityOutlined} color="primary" />
```

## 需要修复的文件

### 高优先级（影响主要页面）

1. ✅ `src/pages/logs.tsx` - 已修复
2. ✅ `src/pages/unlock.tsx` - 已修复
3. ⚠️ `src/pages/profiles.tsx` - 需要修复
4. ⚠️ `src/components/xdp/xdp-config.tsx` - 需要修复
5. ⚠️ `src/components/security/tls-fingerprint-selector.tsx` - 需要修复
6. ⚠️ `src/components/security/security-monitor.tsx` - 需要修复
7. ⚠️ `src/components/security/anti-probe-config.tsx` - 需要修复
8. ⚠️ `src/components/multipath/multipath-config.tsx` - 需要修复

### 中优先级（影响次要页面）

9. `src/components/home/system-info-card.tsx`
10. `src/components/home/ip-info-card.tsx`
11. `src/components/rule/provider-button.tsx`
12. `src/components/proxy/provider-button.tsx`

## 快速修复脚本

### 使用正则表达式批量替换

```bash
# 查找所有没有 fontSize 的图标
grep -r "<[A-Z][a-zA-Z]*Outlined [^f/>]*\/>" src/

# 替换模式（需要手动调整）
# 查找: <(\w+Outlined) ([^f/>]*)/>
# 替换: <$1 $2 fontSize="medium" />
```

### TypeScript 类型检查

修复后运行类型检查：
```bash
pnpm run typecheck
```

## 验证修复

### 1. 开发环境验证

```bash
pnpm dev
```

检查所有页面的图标尺寸是否正常。

### 2. 打包验证

```bash
pnpm build
```

运行打包后的应用，检查图标尺寸。

### 3. 视觉回归测试

对比修复前后的截图，确保：
- ✅ 图标尺寸合理
- ✅ 布局不再混乱
- ✅ 图标与文字对齐
- ✅ 图标在按钮中居中

## 最佳实践

### 1. 新增图标时

```typescript
// 总是设置 fontSize
import { NewIcon } from '@mui/icons-material'

<NewIcon fontSize="medium" />
```

### 2. 使用 ESLint 规则

创建自定义 ESLint 规则检查图标：

```javascript
// .eslintrc.js
module.exports = {
  rules: {
    // 自定义规则：MUI 图标必须有 fontSize
    'mui-icon-font-size': 'error',
  },
}
```

### 3. 代码审查检查清单

- [ ] 所有 MUI 图标都有 `fontSize` 属性
- [ ] IconButton 中的图标使用 `fontSize="inherit"`
- [ ] 标题图标使用 `fontSize="medium"`
- [ ] 小图标使用 `fontSize="small"`

## 常见错误

### 错误 1: 忘记设置 fontSize

```typescript
// ❌ 错误
<SecurityOutlined color="primary" />

// ✅ 正确
<SecurityOutlined color="primary" fontSize="medium" />
```

### 错误 2: IconButton 中使用固定尺寸

```typescript
// ❌ 错误 - 不会随 IconButton size 变化
<IconButton size="small">
  <RefreshOutlined fontSize="medium" />
</IconButton>

// ✅ 正确 - 继承 IconButton 尺寸
<IconButton size="small">
  <RefreshOutlined fontSize="inherit" />
</IconButton>
```

### 错误 3: 在 sx 中设置 fontSize

```typescript
// ⚠️ 不推荐 - 应该使用 fontSize 属性
<SecurityOutlined sx={{ fontSize: 24 }} />

// ✅ 推荐
<SecurityOutlined fontSize="medium" />
```

## 修复进度

- [x] logs.tsx
- [x] unlock.tsx
- [ ] profiles.tsx
- [ ] xdp-config.tsx
- [ ] tls-fingerprint-selector.tsx
- [ ] security-monitor.tsx
- [ ] anti-probe-config.tsx
- [ ] multipath-config.tsx
- [ ] 其他组件...

## 总结

1. **根本原因**: MUI v9 图标默认尺寸变大
2. **推荐方案**: 在主题中设置全局默认尺寸
3. **临时方案**: 批量修复现有代码
4. **最佳实践**: 总是显式设置 `fontSize` 属性

---

**创建日期**: 2026-05-27  
**状态**: 进行中
