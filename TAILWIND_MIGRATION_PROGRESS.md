# Tailwind CSS 迁移进度跟踪

## 📅 开始日期：2026-05-27

---

## ✅ 阶段 1：环境准备（已完成）

### 1.1 依赖安装
- [x] 安装 `tailwindcss`, `postcss`, `autoprefixer`
- [x] 安装 `@headlessui/react` (组件逻辑)
- [x] 安装 `lucide-react` (图标库)
- [x] 安装 `framer-motion` (动画库)

### 1.2 配置文件
- [x] 创建 `tailwind.config.js`
  - 配置主题颜色（primary, secondary, background, card）
  - 配置字体（Outfit, Inter）
  - 配置圆角（card, button, input, dialog）
  - 配置阴影和动画
- [x] 创建 `postcss.config.js`
- [x] 创建 `src/assets/styles/tailwind.css`
  - 基础样式（滚动条、选择文本）
  - 组件样式（card, btn, input）
  - 工具类（text-gradient, glass）

### 1.3 集成到项目
- [x] 更新 `main.tsx` 引入 Tailwind CSS
- [x] 更新 `vite.config.mts` 配置 PostCSS

**状态**：✅ 完成  
**耗时**：1 小时  
**下一步**：创建 Tailwind 组件库

---

## 🚧 阶段 2：创建 Tailwind 组件库（进行中）

### 2.1 基础组件

#### Button 组件
- [ ] 创建 `src/components/tailwind/Button.tsx`
- [ ] 支持 variant: primary, outlined, text
- [ ] 支持 size: small, medium, large
- [ ] 支持 disabled 状态
- [ ] 支持 loading 状态
- [ ] 添加 ARIA 属性

#### TextField 组件
- [ ] 创建 `src/components/tailwind/TextField.tsx`
- [ ] 支持 label, placeholder, error
- [ ] 支持 multiline (textarea)
- [ ] 集成 react-hook-form
- [ ] 添加 ARIA 属性

#### IconButton 组件
- [ ] 创建 `src/components/tailwind/IconButton.tsx`
- [ ] 支持不同尺寸
- [ ] 支持 Tooltip 集成

### 2.2 布局组件

#### Box 组件
- [ ] 创建 `src/components/tailwind/Box.tsx`
- [ ] 替代 MUI Box（使用 div + className）

#### Stack 组件
- [ ] 创建 `src/components/tailwind/Stack.tsx`
- [ ] 支持 direction: row, column
- [ ] 支持 spacing, alignment

#### Grid 组件
- [ ] 创建 `src/components/tailwind/Grid.tsx`
- [ ] 响应式网格布局

### 2.3 反馈组件

#### Dialog 组件
- [ ] 创建 `src/components/tailwind/Dialog.tsx`
- [ ] 使用 Headless UI Dialog
- [ ] 支持 title, description, actions
- [ ] 支持动画过渡

#### Menu 组件
- [ ] 创建 `src/components/tailwind/Menu.tsx`
- [ ] 使用 Headless UI Menu
- [ ] 支持 MenuItem, MenuDivider

#### Tooltip 组件
- [ ] 创建 `src/components/tailwind/Tooltip.tsx`
- [ ] 使用 Headless UI Popover 或自定义实现
- [ ] 支持不同位置（top, bottom, left, right）

#### Skeleton 组件
- [ ] 创建 `src/components/tailwind/Skeleton.tsx`
- [ ] 支持不同形状（text, circular, rectangular）

### 2.4 输入组件

#### Select 组件
- [ ] 创建 `src/components/tailwind/Select.tsx`
- [ ] 使用 Headless UI Listbox
- [ ] 支持单选和多选

#### Switch 组件
- [ ] 创建 `src/components/tailwind/Switch.tsx`
- [ ] 使用 Headless UI Switch

#### Checkbox 组件
- [ ] 创建 `src/components/tailwind/Checkbox.tsx`
- [ ] 添加 ARIA 属性

### 2.5 其他组件

#### Tabs 组件
- [ ] 创建 `src/components/tailwind/Tabs.tsx`
- [ ] 使用 Headless UI Tab

#### Divider 组件
- [ ] 创建 `src/components/tailwind/Divider.tsx`
- [ ] 使用 `<hr>` + Tailwind 类

**状态**：🚧 待开始  
**预计耗时**：5 天  
**优先级**：高

---

## 📋 阶段 3：逐页迁移（待开始）

### 优先级 1：简单页面

#### test.tsx
- [ ] 替换 MUI 组件为 Tailwind 组件
- [ ] 测试功能完整性
- [ ] 测试样式一致性

#### unlock.tsx
- [ ] 替换 MUI 组件为 Tailwind 组件
- [ ] 测试功能完整性

### 优先级 2：中等复杂度

#### settings.tsx
- [ ] 替换 MUI 组件
- [ ] 测试设置项功能

#### rules.tsx
- [ ] 替换 MUI 组件
- [ ] 测试规则列表

#### logs.tsx
- [ ] 替换 MUI 组件
- [ ] 测试日志显示

### 优先级 3：复杂页面

#### home.tsx
- [ ] 替换 MUI 组件
- [ ] 测试所有卡片组件
- [ ] 测试交互功能

#### connections.tsx
- [ ] 替换 MUI 组件
- [ ] 测试连接列表
- [ ] 测试实时更新

#### profiles.tsx
- [ ] 替换 MUI 组件
- [ ] 测试配置文件管理
- [ ] 测试拖拽排序

#### proxies.tsx
- [ ] 替换 MUI 组件
- [ ] 测试代理组显示
- [ ] 测试代理切换

#### advanced.tsx
- [ ] 替换 MUI 组件
- [ ] 测试高级设置

**状态**：📋 待开始  
**预计耗时**：10 天  
**优先级**：中

---

## 🗑️ 阶段 4：移除 MUI 依赖（待开始）

### 4.1 清理代码
- [ ] 删除所有 MUI 导入语句
- [ ] 删除 `src/components/base/base-emotion-style-chain.tsx`
- [ ] 删除 `src/pages/_layout/hooks/use-custom-theme.ts`
- [ ] 清理 `main.tsx` 中的 EmotionStyleChain 和 ThemeProvider

### 4.2 卸载依赖
- [ ] 卸载 `@mui/material`
- [ ] 卸载 `@mui/icons-material`
- [ ] 卸载 `@emotion/react`
- [ ] 卸载 `@emotion/styled`
- [ ] 卸载 `@emotion/cache`
- [ ] 卸载 `@emotion/babel-plugin`

### 4.3 清理配置
- [ ] 移除 `vite.config.mts` 中的 Emotion 配置
- [ ] 清理 SCSS 文件中的 MUI 相关样式

**状态**：🗑️ 待开始  
**预计耗时**：1 天  
**优先级**：低（最后执行）

---

## 🧪 阶段 5：测试和修复（待开始）

### 5.1 功能测试
- [ ] 测试所有页面的基本功能
- [ ] 测试主题切换（亮/暗模式）
- [ ] 测试响应式布局
- [ ] 测试键盘导航
- [ ] 测试无障碍支持（ARIA）

### 5.2 性能测试
- [ ] 测量 Bundle 体积
- [ ] 测量首屏渲染时间
- [ ] 测量运行时性能

### 5.3 兼容性测试
- [ ] Windows 测试
- [ ] macOS 测试
- [ ] Linux 测试

### 5.4 Bug 修复
- [ ] 修复发现的 bug
- [ ] 优化性能问题
- [ ] 调整样式细节

**状态**：🧪 待开始  
**预计耗时**：3 天  
**优先级**：高

---

## 📊 总体进度

| 阶段 | 状态 | 进度 | 预计耗时 | 实际耗时 |
|------|------|------|---------|---------|
| 1. 环境准备 | ✅ 完成 | 100% | 1 天 | 1 小时 |
| 2. 组件库 | 🚧 进行中 | 0% | 5 天 | - |
| 3. 页面迁移 | 📋 待开始 | 0% | 10 天 | - |
| 4. 移除 MUI | 🗑️ 待开始 | 0% | 1 天 | - |
| 5. 测试修复 | 🧪 待开始 | 0% | 3 天 | - |
| **总计** | | **5%** | **20 天** | **1 小时** |

---

## 🎯 当前任务

**下一步**：创建 Button 组件

```bash
# 创建组件目录
mkdir src/components/tailwind

# 创建 Button 组件
# src/components/tailwind/Button.tsx
```

---

## 📝 注意事项

### 迁移原则
1. **保持功能一致**：确保迁移后功能与原来完全一致
2. **保持样式一致**：尽量还原原有的视觉效果
3. **提升无障碍**：确保所有组件都有正确的 ARIA 属性
4. **优化性能**：利用 Tailwind 的零运行时优势

### 风险控制
1. **分支开发**：在独立分支进行迁移
2. **增量迁移**：一次迁移一个页面，逐步验证
3. **保留回滚**：保留 MUI 依赖直到全部迁移完成
4. **充分测试**：每个页面迁移后都要充分测试

### 性能目标
- Bundle 体积减少 30% 以上
- 首屏渲染时间减少 10% 以上
- 无运行时样式注入开销

---

## 🔗 相关文档

- `TAILWIND_MIGRATION_ANALYSIS.md` - 迁移分析文档
- `EMOTION_STYLE_INJECTION_FIX.md` - 原 Emotion 问题修复
- `STYLE_ARCHITECTURE_ANALYSIS.md` - 架构分析

---

**最后更新**：2026-05-27  
**负责人**：Kiro AI Assistant  
**预计完成日期**：2026-06-16 (20 个工作日)
