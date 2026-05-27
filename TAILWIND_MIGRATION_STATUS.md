# Tailwind CSS 迁移 - 当前状态

## 📊 总体进度

**开始日期**：2026-05-27  
**当前阶段**：阶段 3 - 逐页迁移  
**总体完成度**：30%

| 阶段 | 状态 | 进度 | 说明 |
|------|------|------|------|
| 1. 环境准备 | ✅ 完成 | 100% | Tailwind 配置完成 |
| 2. 组件库 | ✅ 完成 | 100% | 13 个组件已创建 |
| 3. 页面迁移 | 🚧 进行中 | 5% | 已创建示例 |
| 4. 移除 MUI | 📋 待开始 | 0% | 等待迁移完成 |
| 5. 测试修复 | 📋 待开始 | 0% | 等待迁移完成 |

---

## ✅ 已完成的工作

### 阶段 1：环境准备（100%）

#### 依赖安装
- ✅ tailwindcss, postcss, autoprefixer
- ✅ @headlessui/react (组件逻辑)
- ✅ lucide-react (图标库)
- ✅ framer-motion (动画库)

#### 配置文件
- ✅ `tailwind.config.js` - 主题配置
- ✅ `postcss.config.js` - PostCSS 配置
- ✅ `src/assets/styles/tailwind.css` - Tailwind 入口文件
- ✅ `vite.config.mts` - 集成 PostCSS
- ✅ `main.tsx` - 引入 Tailwind CSS

---

### 阶段 2：组件库（100%）

#### 已创建的组件（13 个）

| 组件 | 文件 | 功能 | 状态 |
|------|------|------|------|
| **Button** | `Button.tsx` | 3 种变体，3 种尺寸，loading 状态 | ✅ |
| **IconButton** | `IconButton.tsx` | 圆形图标按钮，3 种尺寸 | ✅ |
| **TextField** | `TextField.tsx` | 单行/多行，label，error，ARIA | ✅ |
| **Box** | `Box.tsx` | 替代 MUI Box，支持 as prop | ✅ |
| **Stack** | `Stack.tsx` | 堆叠布局，direction，spacing | ✅ |
| **Grid** | `Grid.tsx` | 网格布局，响应式 | ✅ |
| **Dialog** | `Dialog.tsx` | Headless UI，动画，ARIA | ✅ |
| **Menu** | `Menu.tsx` | Headless UI，键盘导航 | ✅ |
| **Tooltip** | `Tooltip.tsx` | Framer Motion，4 个位置 | ✅ |
| **Skeleton** | `Skeleton.tsx` | 3 种变体，动画 | ✅ |
| **Select** | `Select.tsx` | Headless UI Listbox，ARIA | ✅ |
| **Switch** | `Switch.tsx` | Headless UI Switch | ✅ |
| **Divider** | `Divider.tsx` | 水平/垂直分隔线 | ✅ |

#### 组件特性
- ✅ 完整的 TypeScript 类型定义
- ✅ 完整的 ARIA 无障碍支持
- ✅ 响应式设计
- ✅ 暗色模式支持
- ✅ 动画和过渡效果
- ✅ Forward Ref 支持

---

### 阶段 3：页面迁移（5%）

#### 已创建的示例
- ✅ `test-tailwind.tsx` - test.tsx 的 Tailwind 版本（示例）

#### 迁移要点
```tsx
// 旧的 MUI 导入
import { Box, Button, Grid } from '@mui/material'

// 新的 Tailwind 导入
import { Box, Button, Grid } from '@/components/tailwind'

// MUI sx prop → Tailwind className
<Box sx={{ display: 'flex', gap: 1 }}>
  ↓
<div className="flex gap-1">

// MUI variant="contained" → Tailwind variant="primary"
<Button variant="contained" size="small">
  ↓
<Button variant="primary" size="small">
```

---

## 🚧 当前任务

### 需要迁移的页面（60+ 个文件）

#### 优先级 1：简单页面（2 天）
- [ ] `test.tsx` - 测试页面（已有示例 test-tailwind.tsx）
- [ ] `unlock.tsx` - 解锁页面

#### 优先级 2：中等复杂度（3 天）
- [ ] `settings.tsx` - 设置页面
- [ ] `rules.tsx` - 规则页面
- [ ] `logs.tsx` - 日志页面

#### 优先级 3：复杂页面（5 天）
- [ ] `home.tsx` - 首页（多个卡片组件）
- [ ] `connections.tsx` - 连接页面
- [ ] `profiles.tsx` - 配置文件页面
- [ ] `proxies.tsx` - 代理页面
- [ ] `advanced.tsx` - 高级设置页面

#### 组件文件（需要逐个检查）
- [ ] `src/components/home/*.tsx` - 首页组件
- [ ] `src/components/setting/*.tsx` - 设置组件
- [ ] `src/components/layout/*.tsx` - 布局组件
- [ ] `src/components/ui/*.tsx` - UI 组件
- [ ] 其他组件...

---

## 📋 迁移策略

### 方案 A：渐进式迁移（推荐）

**步骤**：
1. 保留原文件（如 `test.tsx`）
2. 创建新文件（如 `test-tailwind.tsx`）
3. 在路由中切换到新文件
4. 测试功能完整性
5. 删除旧文件

**优势**：
- ✅ 可以随时回滚
- ✅ 新旧对比方便
- ✅ 风险最低

**劣势**：
- ❌ 文件数量翻倍（临时）
- ❌ 需要手动管理两套文件

---

### 方案 B：直接替换（快速但风险高）

**步骤**：
1. 直接修改原文件
2. 替换 MUI 导入为 Tailwind 导入
3. 替换 `sx` prop 为 `className`
4. 测试功能

**优势**：
- ✅ 文件数量不变
- ✅ 迁移速度快

**劣势**：
- ❌ 无法回滚（除非用 Git）
- ❌ 风险较高
- ❌ 难以对比新旧实现

---

### 推荐：方案 A（渐进式迁移）

**理由**：
1. 这是一个大型重构项目（60+ 文件）
2. 需要充分测试每个页面
3. 可能需要多次迭代
4. 保留回滚能力很重要

**实施计划**：
1. 每次迁移 1-2 个页面
2. 充分测试功能和样式
3. 确认无问题后删除旧文件
4. 继续下一个页面

---

## 🎯 下一步行动

### 立即任务（今天）

1. **完成 test.tsx 迁移**
   ```bash
   # 1. 测试 test-tailwind.tsx
   # 2. 在路由中切换
   # 3. 测试功能
   # 4. 删除 test.tsx
   ```

2. **迁移 unlock.tsx**
   - 创建 `unlock-tailwind.tsx`
   - 替换 MUI 组件
   - 测试功能

### 本周任务（3 天）

3. **迁移中等复杂度页面**
   - settings.tsx
   - rules.tsx
   - logs.tsx

### 下周任务（5 天）

4. **迁移复杂页面**
   - home.tsx
   - connections.tsx
   - profiles.tsx
   - proxies.tsx
   - advanced.tsx

---

## ⚠️ 注意事项

### 常见问题

#### 1. Grid 组件的响应式
```tsx
// MUI
<Grid size={{ xs: 6, sm: 4, md: 3, lg: 2 }}>

// Tailwind
<Grid item xs={6} sm={4} md={3} lg={2}>
```

**注意**：Tailwind Grid 使用 12 列系统，需要转换：
- MUI `size={6}` = Tailwind `xs={6}` (col-span-6)
- MUI `size={4}` = Tailwind `sm={4}` (sm:col-span-4)

#### 2. sx prop 转换
```tsx
// MUI
sx={{ display: 'flex', gap: 1, p: 3 }}

// Tailwind
className="flex gap-1 p-3"
```

**常用转换**：
- `display: 'flex'` → `flex`
- `gap: 1` → `gap-1` (4px)
- `p: 3` → `p-3` (12px)
- `mb: 4.5` → `mb-18` (72px, 4.5 * 16px)

#### 3. 图标替换
```tsx
// MUI
import { Close } from '@mui/icons-material'
<Close />

// Lucide React
import { X } from 'lucide-react'
<X className="h-5 w-5" />
```

**常用图标映射**：
- `Close` → `X`
- `Add` → `Plus`
- `Delete` → `Trash2`
- `Edit` → `Pencil`
- `Settings` → `Settings`
- `Check` → `Check`
- `ChevronDown` → `ChevronDown`

---

## 📈 性能监控

### 当前 Bundle 体积
- **总体积**：~1.2MB
- **MUI + Emotion**：~400KB
- **Tailwind**：~100KB
- **当前状态**：三层架构（MUI + Emotion + Tailwind）

### 目标 Bundle 体积
- **总体积**：~0.8MB
- **Tailwind**：~50KB（Tree Shaking 后）
- **预期减少**：400KB (-33%)

### 性能指标
- **首屏渲染**：目标减少 10-15%
- **运行时开销**：目标减少 100%（移除 Emotion）
- **CSS 文件大小**：目标减少 66%

---

## 🔗 相关文档

- `TAILWIND_MIGRATION_ANALYSIS.md` - 迁移分析
- `TAILWIND_MIGRATION_PROGRESS.md` - 进度跟踪
- `TAILWIND_MIGRATION_PHASE1_COMPLETE.md` - 阶段 1 报告
- `TAILWIND_MIGRATION_PHASE2_COMPLETE.md` - 阶段 2 报告
- `EMOTION_STYLE_INJECTION_FIX.md` - Emotion 问题修复

---

## 📝 迁移检查清单

### 每个页面迁移后需要检查

- [ ] 所有 MUI 导入已替换为 Tailwind 导入
- [ ] 所有 `sx` prop 已转换为 `className`
- [ ] 所有 MUI 图标已替换为 Lucide React 图标
- [ ] 页面功能正常（按钮点击、表单提交等）
- [ ] 样式与原版一致（布局、颜色、间距）
- [ ] 响应式布局正常（移动端、平板、桌面）
- [ ] 暗色模式正常
- [ ] 无障碍支持正常（键盘导航、ARIA）
- [ ] 无控制台错误或警告

---

## 🎉 里程碑

- [x] **2026-05-27**: 阶段 1 完成 - 环境准备
- [x] **2026-05-27**: 阶段 2 完成 - 组件库创建
- [ ] **2026-06-03**: 阶段 3 完成 - 页面迁移（预计）
- [ ] **2026-06-04**: 阶段 4 完成 - 移除 MUI（预计）
- [ ] **2026-06-07**: 阶段 5 完成 - 测试修复（预计）
- [ ] **2026-06-10**: 项目完成 - 正式发布（预计）

---

**最后更新**：2026-05-27  
**负责人**：Kiro AI Assistant  
**预计完成日期**：2026-06-10
