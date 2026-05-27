# Tailwind CSS 迁移 - 快速指南

## 🚀 5 分钟快速上手

### 1. 替换导入

```tsx
// ❌ 旧的
import { Box, Button, TextField, Dialog } from '@mui/material'
import { Close, Add } from '@mui/icons-material'

// ✅ 新的
import { Box, Button, TextField, Dialog } from '@/components/tailwind'
import { X, Plus } from 'lucide-react'
```

---

### 2. 替换样式 Prop

```tsx
// ❌ 旧的 MUI sx prop
<Box sx={{ display: 'flex', gap: 2, p: 3, mb: 4 }}>

// ✅ 新的 Tailwind className
<Box className="flex gap-2 p-3 mb-4">
```

---

### 3. 替换组件 Props

```tsx
// ❌ 旧的 MUI
<Button variant="contained" size="small">

// ✅ 新的 Tailwind
<Button variant="primary" size="small">
```

---

## 📋 常用转换表

### Button 组件

| MUI | Tailwind |
|-----|----------|
| `variant="contained"` | `variant="primary"` |
| `variant="outlined"` | `variant="outlined"` |
| `variant="text"` | `variant="text"` |

### Box 组件

| MUI sx | Tailwind className |
|--------|-------------------|
| `display: 'flex'` | `flex` |
| `flexDirection: 'column'` | `flex-col` |
| `gap: 1` | `gap-1` (4px) |
| `gap: 2` | `gap-2` (8px) |
| `p: 1` | `p-1` (4px) |
| `p: 3` | `p-3` (12px) |
| `px: 2` | `px-2` (8px) |
| `py: 3` | `py-3` (12px) |
| `mb: 4` | `mb-4` (16px) |
| `mt: 2` | `mt-2` (8px) |

### Grid 组件

```tsx
// ❌ MUI
<Grid container spacing={2}>
  <Grid size={{ xs: 6, sm: 4, md: 3 }}>

// ✅ Tailwind
<Grid container spacing={2}>
  <Grid item xs={6} sm={4} md={3}>
```

### 图标映射

| MUI Icon | Lucide React |
|----------|--------------|
| `Close` | `X` |
| `Add` | `Plus` |
| `Delete` | `Trash2` |
| `Edit` | `Pencil` |
| `Settings` | `Settings` |
| `Check` | `Check` |
| `ChevronDown` | `ChevronDown` |
| `ChevronUp` | `ChevronUp` |
| `ChevronLeft` | `ChevronLeft` |
| `ChevronRight` | `ChevronRight` |
| `MoreVert` | `MoreVertical` |
| `MoreHoriz` | `MoreHorizontal` |
| `Refresh` | `RefreshCw` |
| `Search` | `Search` |
| `Info` | `Info` |
| `Warning` | `AlertTriangle` |
| `Error` | `AlertCircle` |

---

## 🎨 Tailwind 常用类名

### 布局

```tsx
// Flexbox
className="flex"                    // display: flex
className="flex-col"                // flex-direction: column
className="flex-row"                // flex-direction: row
className="items-center"            // align-items: center
className="justify-between"         // justify-content: space-between
className="gap-2"                   // gap: 8px

// Grid
className="grid grid-cols-12"       // 12 列网格
className="col-span-6"              // 占 6 列
className="sm:col-span-4"           // 小屏占 4 列
```

### 间距

```tsx
// Padding
className="p-2"                     // padding: 8px
className="px-4"                    // padding-left/right: 16px
className="py-3"                    // padding-top/bottom: 12px

// Margin
className="m-2"                     // margin: 8px
className="mx-4"                    // margin-left/right: 16px
className="my-3"                    // margin-top/bottom: 12px
className="mb-4"                    // margin-bottom: 16px
```

### 尺寸

```tsx
className="w-full"                  // width: 100%
className="h-12"                    // height: 48px
className="max-w-md"                // max-width: 28rem
className="min-h-screen"            // min-height: 100vh
```

### 颜色

```tsx
// 背景色
className="bg-primary"              // 主色
className="bg-card-light dark:bg-card-dark"  // 卡片背景

// 文本色
className="text-gray-900 dark:text-gray-100"  // 文本颜色
className="text-primary dark:text-primary-dark-mode"  // 主色文本
```

### 圆角

```tsx
className="rounded"                 // border-radius: 4px
className="rounded-lg"              // border-radius: 8px
className="rounded-button"          // border-radius: 9999px
className="rounded-card"            // border-radius: 32px
```

### 阴影

```tsx
className="shadow"                  // 默认阴影
className="shadow-md"               // 中等阴影
className="shadow-lg"               // 大阴影
className="shadow-card"             // 卡片阴影
```

---

## 🔄 完整迁移示例

### 示例 1：简单按钮组

```tsx
// ❌ 旧的 MUI
<Box sx={{ display: 'flex', gap: 1 }}>
  <Button variant="contained" size="small">
    Save
  </Button>
  <Button variant="outlined" size="small">
    Cancel
  </Button>
</Box>

// ✅ 新的 Tailwind
<div className="flex gap-1">
  <Button variant="primary" size="small">
    Save
  </Button>
  <Button variant="outlined" size="small">
    Cancel
  </Button>
</div>
```

### 示例 2：表单输入

```tsx
// ❌ 旧的 MUI
<TextField
  label="Username"
  variant="outlined"
  fullWidth
  error={!!errors.username}
  helperText={errors.username?.message}
/>

// ✅ 新的 Tailwind
<TextField
  label="Username"
  error={errors.username?.message}
  className="w-full"
/>
```

### 示例 3：对话框

```tsx
// ❌ 旧的 MUI
<Dialog open={open} onClose={onClose}>
  <DialogTitle>Confirm</DialogTitle>
  <DialogContent>
    Are you sure?
  </DialogContent>
  <DialogActions>
    <Button onClick={onClose}>Cancel</Button>
    <Button onClick={onConfirm} variant="contained">
      Confirm
    </Button>
  </DialogActions>
</Dialog>

// ✅ 新的 Tailwind
<Dialog
  open={open}
  onClose={onClose}
  title="Confirm"
  actions={
    <>
      <Button onClick={onClose} variant="outlined">
        Cancel
      </Button>
      <Button onClick={onConfirm} variant="primary">
        Confirm
      </Button>
    </>
  }
>
  Are you sure?
</Dialog>
```

### 示例 4：网格布局

```tsx
// ❌ 旧的 MUI
<Grid container spacing={2}>
  <Grid size={{ xs: 12, sm: 6, md: 4 }}>
    <Card>Content 1</Card>
  </Grid>
  <Grid size={{ xs: 12, sm: 6, md: 4 }}>
    <Card>Content 2</Card>
  </Grid>
</Grid>

// ✅ 新的 Tailwind
<Grid container spacing={2}>
  <Grid item xs={12} sm={6} md={4}>
    <div className="card">Content 1</div>
  </Grid>
  <Grid item xs={12} sm={6} md={4}>
    <div className="card">Content 2</div>
  </Grid>
</Grid>
```

---

## 🎯 迁移步骤（每个文件）

### 1. 创建新文件
```bash
# 例如迁移 settings.tsx
cp src/pages/settings.tsx src/pages/settings-tailwind.tsx
```

### 2. 替换导入
```tsx
// 在 settings-tailwind.tsx 中
// 查找所有 '@mui/material' 导入
// 替换为 '@/components/tailwind'
```

### 3. 替换样式
```tsx
// 查找所有 sx={{ ... }}
// 替换为 className="..."
```

### 4. 替换图标
```tsx
// 查找所有 '@mui/icons-material' 导入
// 替换为 'lucide-react'
```

### 5. 测试功能
```bash
# 在路由中切换到新文件
# 测试所有功能
# 检查样式是否一致
```

### 6. 删除旧文件
```bash
# 确认无问题后
rm src/pages/settings.tsx
mv src/pages/settings-tailwind.tsx src/pages/settings.tsx
```

---

## ⚠️ 常见陷阱

### 1. spacing 值不同

```tsx
// ❌ 错误：MUI spacing 是 8px 的倍数
sx={{ gap: 2 }}  // 16px

// ✅ 正确：Tailwind spacing 是 4px 的倍数
className="gap-2"  // 8px
className="gap-4"  // 16px (等同于 MUI gap: 2)
```

### 2. Grid 列数系统

```tsx
// ❌ 错误：MUI 使用 12 列，但 size prop 直接指定列数
<Grid size={6}>  // 占 6 列

// ✅ 正确：Tailwind 也使用 12 列
<Grid item xs={6}>  // 占 6 列
```

### 3. 图标尺寸

```tsx
// ❌ 错误：Lucide 图标默认没有尺寸
<X />  // 可能很大或很小

// ✅ 正确：始终指定尺寸
<X className="h-5 w-5" />  // 20x20px
```

### 4. 暗色模式

```tsx
// ❌ 错误：忘记暗色模式
className="bg-white text-black"

// ✅ 正确：同时指定亮色和暗色
className="bg-white dark:bg-gray-900 text-black dark:text-white"
```

---

## 🔍 调试技巧

### 1. 检查样式是否生效

```tsx
// 添加明显的背景色测试
className="bg-red-500"  // 应该显示红色背景
```

### 2. 检查 Tailwind 是否加载

```javascript
// 在浏览器控制台运行
document.querySelector('style')?.textContent.includes('tailwind')
// 应该返回 true
```

### 3. 检查暗色模式

```javascript
// 在浏览器控制台运行
document.documentElement.classList.contains('dark')
// 暗色模式下应该返回 true
```

### 4. 对比新旧实现

```bash
# 使用 Git diff 对比
git diff src/pages/settings.tsx src/pages/settings-tailwind.tsx
```

---

## 📚 参考资源

### Tailwind CSS 文档
- [官方文档](https://tailwindcss.com/docs)
- [Cheat Sheet](https://nerdcave.com/tailwind-cheat-sheet)

### Headless UI 文档
- [Dialog](https://headlessui.com/react/dialog)
- [Menu](https://headlessui.com/react/menu)
- [Listbox](https://headlessui.com/react/listbox)
- [Switch](https://headlessui.com/react/switch)

### Lucide React 文档
- [图标搜索](https://lucide.dev/icons/)
- [使用指南](https://lucide.dev/guide/packages/lucide-react)

---

## 🎉 完成检查清单

每个文件迁移完成后，确认：

- [ ] 所有 MUI 导入已替换
- [ ] 所有 sx prop 已转换为 className
- [ ] 所有图标已替换
- [ ] 页面功能正常
- [ ] 样式与原版一致
- [ ] 响应式布局正常
- [ ] 暗色模式正常
- [ ] 无控制台错误

---

**快速指南版本**：1.0  
**最后更新**：2026-05-27  
**作者**：Kiro AI Assistant
