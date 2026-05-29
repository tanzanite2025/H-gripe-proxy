# Tailwind 迁移修复计划

## 当前状态
- 已移除所有 MUI 导入
- 已修复基本的组件属性（variant, size等）
- 剩余约400+个TypeScript错误

## 主要问题分类

### 1. MUI 图标未替换 (约150个错误)
需要将 @mui/icons-material 替换为 lucide-react

**常见映射：**
- `CloseRounded` → `X`
- `RefreshOutlined` → `RefreshCw`
- `ErrorOutlined` → `AlertCircle`
- `CheckCircleOutlined` → `CheckCircle`
- `WarningOutlined` → `AlertTriangle`
- `InfoOutlined` → `Info`
- `DeleteOutlined` → `Trash2`
- `VisibilityOutlined` → `Eye`
- `VisibilityOffOutlined` → `EyeOff`
- `ArrowUpwardRounded` → `ArrowUp`
- `ArrowDownwardRounded` → `ArrowDown`
- `LocationOnOutlined` → `MapPin`
- `SecurityOutlined` → `Shield`
- `SpeedOutlined` → `Gauge`
- `SaveOutlined` → `Save`
- `ComputerRounded` → `Monitor`
- `CloudUploadRounded` → `CloudUpload`
- `CloudDownloadRounded` → `CloudDownload`
- `MemoryRounded` → `Cpu`
- `LinkRounded` → `Link`

### 2. @emotion 相关代码
**文件：** `src/components/base/base-emotion-style-chain.tsx`

**解决方案：** 完全删除此文件，或者重写为不依赖@emotion的版本

### 3. SelectChangeEvent 类型问题
**文件：**
- `src/components/home/current-proxy-card/components/proxy-selectors.tsx`
- `src/components/home/current-proxy-card/hooks/use-current-proxy-data.ts`

**问题：** 使用了MUI的 `SelectChangeEvent` 类型

**解决方案：** 
```typescript
// 旧代码
onChange={(event: SelectChangeEvent<string>) => ...}

// 新代码
onChange={(value: string | number) => ...}
```

### 4. DnD Kit 导入缺失
**文件：** `src/components/connection/connection-column-manager.tsx`

**解决方案：** 添加导入
```typescript
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
```

### 5. React Hooks 导入缺失
**文件：** `src/components/home/enhanced-canvas-traffic-graph/hooks/use-graph-renderer.ts`

**解决方案：** 添加导入
```typescript
import { useRef, useCallback, useEffect } from 'react';
```

### 6. 组件属性不兼容

#### Chip 组件
- 移除 `onDelete` 属性（Tailwind版本不支持）
- 移除 `onClick` 属性

#### TextField 组件
- `onChange` 事件处理器需要从 `e.target.value` 改为直接使用 `e.target.value`

#### Select 组件
- 移除 `labelId` 属性
- `onChange` 从 `(event) => event.target.value` 改为 `(value) => value`

#### ListItem 相关
- 移除 `secondaryAction` 属性
- 移除 `style` 属性
- 移除 `title` 属性（ListItemButton）
- 移除 `onClick` 属性（ListItemText）
- 移除 `secondaryClassName` 属性（ListItemText）

#### Snackbar 组件
- 移除 `message` 属性
- 移除 `style` 属性

#### Dialog 组件
- 移除 `disableEnforceFocus` 属性

#### Paper 组件
- 移除 `onClick` 属性

### 7. 自定义组件问题

#### base-search-box.tsx
- `matchCaseIcon` 和 `matchWholeWordIcon` 不是有效的JSX元素
- 需要重写为使用lucide-react图标

#### base-split-chip-editor.tsx
- Chip的 `onDelete` 属性不支持
- 类型不匹配问题

## 修复优先级

### 高优先级（阻塞编译）
1. ✅ 移除所有 MUI 导入
2. ✅ 修复基本组件属性
3. 🔄 替换所有 MUI 图标为 lucide-react
4. 🔄 修复 SelectChangeEvent 类型
5. 🔄 添加缺失的导入（DnD Kit, React hooks）

### 中优先级（功能影响）
6. 删除或重写 base-emotion-style-chain.tsx
7. 修复 base-search-box.tsx 的图标问题
8. 修复 base-split-chip-editor.tsx 的 Chip 问题

### 低优先级（优化）
9. 清理未使用的代码
10. 优化类型定义

## 下一步行动

1. 创建图标映射脚本，批量替换MUI图标
2. 修复SelectChangeEvent类型问题
3. 添加缺失的导入
4. 处理特殊组件（base-search-box, base-split-chip-editor）
5. 删除base-emotion-style-chain.tsx
6. 运行完整的typecheck验证

## 预计工作量
- 图标替换：自动化脚本 + 手动调整 = 30分钟
- 类型修复：15分钟
- 导入修复：10分钟
- 特殊组件：30分钟
- 总计：约1.5小时
