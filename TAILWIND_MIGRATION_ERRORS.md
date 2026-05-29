# Tailwind 迁移剩余错误

## 错误统计
- 初始错误数：741 个
- 主要错误类型：
  1. ✅ TextField onChange 事件类型不匹配 (部分修复)
  2. ✅ 缺失的 Tailwind 组件（MenuItem）- 已修复
  3. ⚠️ Grid size 属性类型错误
  4. ⚠️ 其他 TextField onChange 需要批量修复

## 已修复的文件
✅ base-search-box.tsx - 移除 InputAdornment，修复 aria-label
✅ profile-viewer-ui.tsx - 移除 InputAdornment，使用 endAdornment
✅ auto-backup-settings.tsx - 替换 ListItem/ListItemText 为 div
✅ backup-config.tsx - 移除 InputAdornment
✅ backup-main.tsx - 修复 ListItemText 语法错误
✅ clash-port.tsx - 修复所有 ListItemText 的 }} 错误 + TextField onChange
✅ layout-config.tsx - 移除 InputAdornment
✅ lite-mode.tsx - 移除 InputAdornment + TextField onChange
✅ layout.tsx - 修复 Menu 闭合标签
✅ proxy-selectors.tsx - 替换 FormControl/InputLabel 为 div/label + MenuItem 导入
✅ security-config-panel.tsx - 修复 TextField onChange
✅ xdp-config-panel.tsx - 修复 TextField onChange (2处)

## 待修复的主要问题

### 1. TextField onChange 类型问题 (批量修复需要)
需要将 `onChange={(value) =>` 改为 `onChange={(e) =>` 并使用 `e.target.value`

受影响的文件（约50+个）：
- xdp-config-ui.tsx
- webui-item.tsx
- password-input.tsx
- tunnels-config.tsx
- tun-config.tsx
- system-proxy-ui.tsx
- controller.tsx
- external-cors.tsx
- anti-probe-config-ui.tsx
- header-sanitization-config.tsx
- 等等...

### 2. Select onChange 类型问题
Select 组件也有类似问题，需要统一处理

### 3. Grid 组件问题
Grid 的 size 属性应该使用新的 API：
- profiles.tsx
- settings.tsx
- test.tsx
- unlock.tsx

### 4. 其他组件属性不兼容
- Select 的 fullWidth、MenuProps、renderValue 属性
- TextField 的 slotProps 属性

## 建议的修复策略

### 方案 1：创建包装组件
创建一个 TextField 包装器，自动处理 onChange 事件转换：
```tsx
// 支持两种 onChange 签名
onChange?: ((e: ChangeEvent<HTMLInputElement>) => void) | ((value: string) => void)
```

### 方案 2：批量查找替换
使用正则表达式批量替换所有 TextField/Select 的 onChange

### 方案 3：渐进式修复
优先修复编译错误最多的文件，其他文件逐步修复

## 当前状态
- 已修复约 15 个文件的 MUI 残留问题
- TextField onChange 类型问题需要批量处理
- 建议采用方案 1（创建包装组件）来减少代码改动
