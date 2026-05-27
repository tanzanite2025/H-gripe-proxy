# Requirements Document

## Introduction

本文档定义了将 Settings 组件从 MUI (Material-UI) 迁移到 Tailwind CSS 的需求。这是主应用 Tailwind CSS 迁移的第二阶段，旨在完成整个应用的样式架构统一。

主应用的 10 个页面和 Layout 已成功迁移到 Tailwind CSS，实现了 80% 的 bundle size 减少（2.5MB → 500KB）和单层架构。Settings 组件是最后一个使用 MUI 的主要模块，包含 41 个文件，其中约 35 个文件使用 MUI 组件。

## Glossary

- **Settings_Module**: Settings 组件模块，包含所有设置相关的 UI 组件
- **MUI_Component**: Material-UI 框架提供的 React 组件
- **Tailwind_Component**: 项目中已创建的 23 个基于 Tailwind CSS 的自定义组件
- **Migration_Tool**: 用于自动化迁移的脚本工具
- **Theme_System**: 应用的主题系统，支持深色/浅色模式切换
- **CSS_Variable**: CSS 自定义属性，用于主题颜色和样式管理
- **Bundle_Size**: 应用打包后的文件大小
- **Component_Tree**: Settings 组件的层级结构，包括顶层组件和子组件
- **Functional_Parity**: 迁移后功能与迁移前完全一致
- **Visual_Consistency**: 迁移后视觉效果与迁移前保持一致或改善

## Requirements

### Requirement 1: 迁移顶层 Settings 组件

**User Story:** 作为开发者，我希望迁移 7 个顶层 Settings 组件到 Tailwind CSS，这样可以统一样式架构并减少依赖。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `setting-verge-basic.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `setting-verge-advanced.tsx` 从 MUI 到 Tailwind_Component
3. THE Migration_Tool SHALL 迁移 `setting-clash.tsx` 从 MUI 到 Tailwind_Component
4. THE Migration_Tool SHALL 迁移 `setting-system.tsx` 从 MUI 到 Tailwind_Component
5. THE Migration_Tool SHALL 迁移 `dns-stats-card.tsx` 从 MUI 到 Tailwind_Component
6. THE Migration_Tool SHALL 迁移 `dns-routing-card.tsx` 从 MUI 到 Tailwind_Component
7. THE Migration_Tool SHALL 迁移 `dns-leak-protection-card.tsx` 从 MUI 到 Tailwind_Component
8. THE Migration_Tool SHALL 迁移 `tor-config-card.tsx` 从 MUI 到 Tailwind_Component
9. FOR ALL 迁移的组件，THE Migration_Tool SHALL 保持 Functional_Parity
10. FOR ALL 迁移的组件，THE Migration_Tool SHALL 保持 Visual_Consistency

### Requirement 2: 迁移 WebUI 子组件

**User Story:** 作为开发者，我希望迁移 WebUI 相关的子组件到 Tailwind CSS，这样可以统一 Web UI 配置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/webui/webui-item.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/webui/webui-config.tsx` 从 MUI 到 Tailwind_Component
3. WHEN 用户配置 Web UI 设置时，THE Settings_Module SHALL 正确显示和保存配置
4. FOR ALL WebUI 组件，THE Migration_Tool SHALL 保持表单验证逻辑不变

### Requirement 3: 迁移 Theme 子组件

**User Story:** 作为开发者，我希望迁移主题配置子组件到 Tailwind CSS，这样可以统一主题设置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/theme/theme-mode-switch.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/theme/theme-config.tsx` 从 MUI 到 Tailwind_Component
3. WHEN 用户切换主题模式时，THE Theme_System SHALL 正确应用深色或浅色主题
4. THE Theme_System SHALL 使用 CSS_Variable 管理主题颜色
5. FOR ALL 主题组件，THE Migration_Tool SHALL 保持主题切换动画效果

### Requirement 4: 迁移 Network 子组件

**User Story:** 作为开发者，我希望迁移网络配置子组件到 Tailwind CSS，这样可以统一网络设置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/network/tunnels-config.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/network/tun-config.tsx` 从 MUI 到 Tailwind_Component
3. THE Migration_Tool SHALL 迁移 `components/network/network-interface.tsx` 从 MUI 到 Tailwind_Component
4. THE Migration_Tool SHALL 迁移 `components/network/external-cors.tsx` 从 MUI 到 Tailwind_Component
5. THE Migration_Tool SHALL 迁移 `components/network/controller.tsx` 从 MUI 到 Tailwind_Component
6. WHEN 用户配置网络设置时，THE Settings_Module SHALL 正确验证和保存配置
7. FOR ALL 网络组件，THE Migration_Tool SHALL 保持输入验证和错误提示逻辑

### Requirement 5: 迁移 Proxy 子组件

**User Story:** 作为开发者，我希望迁移代理配置子组件到 Tailwind CSS，这样可以统一代理设置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/proxy/system-proxy.tsx` 从 MUI 到 Tailwind_Component
2. WHEN 用户配置系统代理时，THE Settings_Module SHALL 正确应用代理设置
3. THE Migration_Tool SHALL 保持代理配置的表单验证逻辑

### Requirement 6: 迁移 Misc 子组件

**User Story:** 作为开发者，我希望迁移杂项配置子组件到 Tailwind CSS，这样可以统一其他设置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/misc/misc-config.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/misc/update-config.tsx` 从 MUI 到 Tailwind_Component
3. THE Migration_Tool SHALL 迁移 `components/misc/stack-mode-switch.tsx` 从 MUI 到 Tailwind_Component
4. THE Migration_Tool SHALL 迁移 `components/misc/lite-mode.tsx` 从 MUI 到 Tailwind_Component
5. THE Migration_Tool SHALL 迁移 `components/misc/layout-config.tsx` 从 MUI 到 Tailwind_Component
6. THE Migration_Tool SHALL 迁移 `components/misc/config-editor.tsx` 从 MUI 到 Tailwind_Component
7. WHEN 用户配置应用更新时，THE Settings_Module SHALL 正确显示更新进度
8. FOR ALL 杂项组件，THE Migration_Tool SHALL 保持配置持久化逻辑

### Requirement 7: 迁移 Hotkey 子组件

**User Story:** 作为开发者，我希望迁移快捷键配置子组件到 Tailwind CSS，这样可以统一快捷键设置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/hotkey/hotkey-input.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/hotkey/hotkey-config.tsx` 从 MUI 到 Tailwind_Component
3. WHEN 用户设置快捷键时，THE Settings_Module SHALL 正确捕获键盘输入
4. WHEN 快捷键冲突时，THE Settings_Module SHALL 显示警告提示
5. THE Migration_Tool SHALL 保持快捷键验证和冲突检测逻辑

### Requirement 8: 迁移 Clash DNS 配置子组件

**User Story:** 作为开发者，我希望迁移 Clash DNS 配置子组件到 Tailwind CSS，这样可以统一 DNS 设置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/clash/dns-config/index.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/clash/dns-config/components/dns-general-fields.tsx` 从 MUI 到 Tailwind_Component
3. THE Migration_Tool SHALL 迁移 `components/clash/dns-config/components/dns-nameserver-fields.tsx` 从 MUI 到 Tailwind_Component
4. THE Migration_Tool SHALL 迁移 `components/clash/dns-config/components/dns-fallback-fields.tsx` 从 MUI 到 Tailwind_Component
5. THE Migration_Tool SHALL 迁移 `components/clash/dns-config/components/dns-hosts-fields.tsx` 从 MUI 到 Tailwind_Component
6. WHEN 用户配置 DNS 设置时，THE Settings_Module SHALL 正确验证 DNS 服务器地址格式
7. WHEN 用户保存 DNS 配置时，THE Settings_Module SHALL 正确生成 YAML 配置
8. FOR ALL DNS 组件，THE Migration_Tool SHALL 保持 DNS 配置解析和序列化逻辑

### Requirement 9: 迁移 Clash Core 和 Port 子组件

**User Story:** 作为开发者，我希望迁移 Clash 核心配置子组件到 Tailwind CSS，这样可以统一 Clash 设置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/clash/clash-core.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/clash/clash-port.tsx` 从 MUI 到 Tailwind_Component
3. WHEN 用户切换 Clash 核心时，THE Settings_Module SHALL 正确重启 Clash 服务
4. WHEN 用户修改端口配置时，THE Settings_Module SHALL 验证端口号范围（1-65535）
5. THE Migration_Tool SHALL 保持 Clash 核心切换和端口配置的业务逻辑

### Requirement 10: 迁移 Shared 子组件

**User Story:** 作为开发者，我希望迁移共享子组件到 Tailwind CSS，这样可以统一所有设置项的基础样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/shared/setting-item.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/shared/password-input.tsx` 从 MUI 到 Tailwind_Component
3. FOR ALL 使用 `setting-item.tsx` 的组件，THE Migration_Tool SHALL 保持布局和交互一致
4. WHEN 用户输入密码时，THE Settings_Module SHALL 支持显示/隐藏密码功能

### Requirement 11: 迁移 Backup 子组件

**User Story:** 作为开发者，我希望迁移备份配置子组件到 Tailwind CSS，这样可以统一备份设置界面的样式。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 迁移 `components/backup/backup-main.tsx` 从 MUI 到 Tailwind_Component
2. THE Migration_Tool SHALL 迁移 `components/backup/backup-config.tsx` 从 MUI 到 Tailwind_Component
3. THE Migration_Tool SHALL 迁移 `components/backup/backup-history.tsx` 从 MUI 到 Tailwind_Component
4. THE Migration_Tool SHALL 迁移 `components/backup/backup-webdav-dialog.tsx` 从 MUI 到 Tailwind_Component
5. THE Migration_Tool SHALL 迁移 `components/backup/auto-backup-settings.tsx` 从 MUI 到 Tailwind_Component
6. WHEN 用户创建备份时，THE Settings_Module SHALL 正确保存配置快照
7. WHEN 用户恢复备份时，THE Settings_Module SHALL 正确加载历史配置
8. FOR ALL 备份组件，THE Migration_Tool SHALL 保持备份和恢复的业务逻辑

### Requirement 12: 使用现有 Tailwind 组件库

**User Story:** 作为开发者，我希望迁移过程中使用已创建的 23 个 Tailwind 组件，这样可以保持样式一致性并减少重复工作。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 使用 `Button` 替换所有 MUI `Button` 组件
2. THE Migration_Tool SHALL 使用 `TextField` 替换所有 MUI `TextField` 和 `Input` 组件
3. THE Migration_Tool SHALL 使用 `Select` 替换所有 MUI `Select` 和 `MenuItem` 组件
4. THE Migration_Tool SHALL 使用 `Switch` 替换所有 MUI `Switch` 组件
5. THE Migration_Tool SHALL 使用 `Dialog` 替换所有 MUI `Dialog` 组件
6. THE Migration_Tool SHALL 使用 `Tooltip` 替换所有 MUI `Tooltip` 组件
7. THE Migration_Tool SHALL 使用 `IconButton` 替换所有 MUI `IconButton` 组件
8. THE Migration_Tool SHALL 使用 `Box` 替换所有 MUI `Box` 组件
9. THE Migration_Tool SHALL 使用 `Stack` 替换所有 MUI `Stack` 组件
10. THE Migration_Tool SHALL 使用 `Divider` 替换所有 MUI `Divider` 组件
11. THE Migration_Tool SHALL 使用 `Typography` 替换所有 MUI `Typography` 组件
12. THE Migration_Tool SHALL 使用 `CircularProgress` 替换所有 MUI `CircularProgress` 组件
13. THE Migration_Tool SHALL 使用 `Alert` 替换所有 MUI `Alert` 组件
14. THE Migration_Tool SHALL 使用 `Chip` 替换所有 MUI `Chip` 组件
15. THE Migration_Tool SHALL 使用 `ButtonGroup` 替换所有 MUI `ButtonGroup` 组件
16. THE Migration_Tool SHALL 使用 `Tabs` 和 `Tab` 替换所有 MUI `Tabs` 和 `Tab` 组件
17. THE Migration_Tool SHALL 使用 `Card` 替换所有 MUI `Card` 组件
18. THE Migration_Tool SHALL 使用 `Skeleton` 替换所有 MUI `Skeleton` 组件
19. THE Migration_Tool SHALL 使用 `Menu` 和 `MenuItem` 替换所有 MUI `Menu` 和 `MenuItem` 组件
20. THE Migration_Tool SHALL 使用 `Collapse` 或自定义实现替换所有 MUI `Collapse` 组件

### Requirement 13: 图标迁移

**User Story:** 作为开发者，我希望将 MUI Icons 迁移到 Lucide React，这样可以减少依赖并使用更现代的图标库。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 使用 Lucide React 图标替换所有 `@mui/icons-material` 图标
2. FOR ALL 图标替换，THE Migration_Tool SHALL 保持图标语义一致（例如：`HomeRounded` → `Home`）
3. THE Migration_Tool SHALL 使用 `size` 属性替换 MUI 的 `fontSize` 属性
4. WHEN 图标没有直接对应时，THE Migration_Tool SHALL 选择语义最接近的 Lucide 图标

### Requirement 14: 样式属性迁移

**User Story:** 作为开发者，我希望将 MUI 的 `sx` 属性迁移到 Tailwind 的 `className`，这样可以使用编译时 CSS 而非运行时 CSS-in-JS。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 将所有 `sx` 属性转换为 Tailwind `className`
2. THE Migration_Tool SHALL 将 MUI 主题变量（如 `theme.palette.primary.main`）转换为 CSS_Variable
3. THE Migration_Tool SHALL 将 MUI 间距单位（如 `p: 2`）转换为 Tailwind 间距类（如 `p-8`）
4. THE Migration_Tool SHALL 将 MUI 颜色函数（如 `alpha(color, 0.5)`）转换为 Tailwind 透明度类或 CSS_Variable
5. THE Migration_Tool SHALL 将 MUI 响应式断点（如 `{ xs: 12, md: 6 }`）转换为 Tailwind 响应式类（如 `w-full md:w-1/2`）
6. FOR ALL 复杂的 `sx` 属性，THE Migration_Tool SHALL 保持视觉效果一致

### Requirement 15: 主题系统兼容

**User Story:** 作为用户，我希望迁移后的 Settings 组件支持深色/浅色主题切换，这样可以保持与主应用一致的主题体验。

#### Acceptance Criteria

1. THE Settings_Module SHALL 使用 CSS_Variable 管理主题颜色
2. WHEN 用户切换主题时，THE Settings_Module SHALL 立即应用新主题颜色
3. THE Settings_Module SHALL 支持自定义主题颜色配置
4. THE Settings_Module SHALL 在深色模式下使用深色背景和浅色文字
5. THE Settings_Module SHALL 在浅色模式下使用浅色背景和深色文字
6. FOR ALL 主题颜色，THE Settings_Module SHALL 保持足够的对比度以确保可读性

### Requirement 16: 响应式布局保持

**User Story:** 作为用户，我希望迁移后的 Settings 组件在不同屏幕尺寸下正常显示，这样可以在各种设备上使用应用。

#### Acceptance Criteria

1. THE Settings_Module SHALL 在桌面屏幕（≥1024px）上使用多列布局
2. THE Settings_Module SHALL 在平板屏幕（768px-1023px）上使用两列布局
3. THE Settings_Module SHALL 在移动屏幕（<768px）上使用单列布局
4. FOR ALL 响应式断点，THE Settings_Module SHALL 保持内容可读性和可操作性
5. THE Settings_Module SHALL 使用 Tailwind 响应式前缀（`sm:`, `md:`, `lg:`, `xl:`）实现响应式布局

### Requirement 17: 动画效果保持

**User Story:** 作为用户，我希望迁移后的 Settings 组件保持流畅的动画效果，这样可以提供良好的用户体验。

#### Acceptance Criteria

1. THE Settings_Module SHALL 使用 Framer Motion 或 Tailwind 过渡类实现动画效果
2. WHEN 对话框打开时，THE Settings_Module SHALL 显示淡入和缩放动画
3. WHEN 折叠面板展开时，THE Settings_Module SHALL 显示平滑的高度过渡动画
4. WHEN 按钮悬停时，THE Settings_Module SHALL 显示颜色和阴影过渡效果
5. FOR ALL 动画，THE Settings_Module SHALL 使用合理的持续时间（100ms-300ms）

### Requirement 18: 表单验证保持

**User Story:** 作为用户，我希望迁移后的 Settings 组件保持表单验证功能，这样可以防止输入无效配置。

#### Acceptance Criteria

1. WHEN 用户输入无效端口号时，THE Settings_Module SHALL 显示错误提示
2. WHEN 用户输入无效 IP 地址时，THE Settings_Module SHALL 显示错误提示
3. WHEN 用户输入无效 URL 时，THE Settings_Module SHALL 显示错误提示
4. WHEN 用户输入空必填字段时，THE Settings_Module SHALL 显示错误提示
5. THE Settings_Module SHALL 在用户修正错误后自动清除错误提示
6. FOR ALL 表单验证，THE Migration_Tool SHALL 保持原有验证逻辑不变

### Requirement 19: 无障碍性保持

**User Story:** 作为使用辅助技术的用户，我希望迁移后的 Settings 组件保持无障碍性，这样可以使用屏幕阅读器等工具操作应用。

#### Acceptance Criteria

1. THE Settings_Module SHALL 为所有交互元素提供适当的 ARIA 标签
2. THE Settings_Module SHALL 支持键盘导航（Tab、Enter、Escape）
3. THE Settings_Module SHALL 为表单字段提供关联的 `<label>` 元素
4. THE Settings_Module SHALL 为图标按钮提供 `aria-label` 属性
5. THE Settings_Module SHALL 为对话框提供 `role="dialog"` 和 `aria-modal="true"` 属性
6. FOR ALL 无障碍特性，THE Migration_Tool SHALL 保持或改善原有无障碍性

### Requirement 20: 性能优化

**User Story:** 作为开发者，我希望迁移后的 Settings 组件减少 bundle size 并提高性能，这样可以加快应用加载速度。

#### Acceptance Criteria

1. WHEN 迁移完成后，THE Settings_Module SHALL 不再依赖 `@mui/material` 包
2. WHEN 迁移完成后，THE Settings_Module SHALL 不再依赖 `@emotion/react` 和 `@emotion/styled` 包
3. THE Settings_Module SHALL 使用编译时 CSS 而非运行时 CSS-in-JS
4. THE Settings_Module SHALL 减少至少 50% 的组件相关 bundle size
5. THE Settings_Module SHALL 在初始渲染时不注入运行时样式标签
6. FOR ALL 组件，THE Migration_Tool SHALL 使用 React.memo 或 useMemo 优化不必要的重渲染

### Requirement 21: 测试覆盖

**User Story:** 作为开发者，我希望迁移后的 Settings 组件通过所有功能测试，这样可以确保迁移没有引入回归问题。

#### Acceptance Criteria

1. WHEN 迁移完成后，THE Settings_Module SHALL 通过所有现有的单元测试
2. WHEN 迁移完成后，THE Settings_Module SHALL 通过所有现有的集成测试
3. THE Settings_Module SHALL 通过手动功能测试（保存配置、加载配置、切换主题）
4. THE Settings_Module SHALL 通过视觉回归测试（截图对比）
5. FOR ALL 关键功能，THE Migration_Tool SHALL 创建新的测试用例以覆盖迁移后的代码

### Requirement 22: 文档更新

**User Story:** 作为开发者，我希望更新相关文档以反映迁移后的架构，这样可以帮助团队成员理解新的代码结构。

#### Acceptance Criteria

1. THE Migration_Tool SHALL 更新 `TAILWIND_MIGRATION_PROGRESS.md` 以反映 Settings 组件迁移状态
2. THE Migration_Tool SHALL 创建 `SETTINGS_MIGRATION_COMPLETE.md` 文档记录迁移详情
3. THE Migration_Tool SHALL 更新 `TAILWIND_COMPONENT_LIBRARY.md` 以包含 Settings 组件使用示例
4. THE Migration_Tool SHALL 更新 `README.md` 以移除 MUI 相关说明
5. FOR ALL 迁移的组件，THE Migration_Tool SHALL 在代码注释中说明主要变更

### Requirement 23: 向后兼容性

**User Story:** 作为开发者，我希望迁移过程中保持 API 兼容性，这样可以避免破坏其他依赖 Settings 组件的代码。

#### Acceptance Criteria

1. THE Settings_Module SHALL 保持所有公共组件的 props 接口不变
2. THE Settings_Module SHALL 保持所有导出的类型定义不变
3. THE Settings_Module SHALL 保持所有事件回调的签名不变
4. WHEN 组件 props 需要变更时，THE Migration_Tool SHALL 提供向后兼容的适配层
5. FOR ALL 公共 API，THE Migration_Tool SHALL 在变更前评估影响范围

### Requirement 24: 渐进式迁移支持

**User Story:** 作为开发者，我希望支持渐进式迁移，这样可以分批次完成迁移而不影响应用稳定性。

#### Acceptance Criteria

1. THE Settings_Module SHALL 支持 MUI 组件和 Tailwind_Component 共存
2. THE Migration_Tool SHALL 允许按子模块（webui、theme、network 等）分批迁移
3. WHEN 部分组件已迁移时，THE Settings_Module SHALL 正常运行
4. THE Migration_Tool SHALL 提供迁移进度跟踪机制
5. FOR ALL 迁移批次，THE Migration_Tool SHALL 确保每个批次完成后应用可正常构建和运行

### Requirement 25: 代码质量保持

**User Story:** 作为开发者，我希望迁移后的代码保持高质量标准，这样可以确保代码可维护性和可读性。

#### Acceptance Criteria

1. THE Settings_Module SHALL 通过 ESLint 代码检查
2. THE Settings_Module SHALL 通过 TypeScript 类型检查
3. THE Settings_Module SHALL 通过 Biome 格式化检查
4. THE Settings_Module SHALL 保持一致的代码风格（缩进、命名、注释）
5. THE Settings_Module SHALL 移除所有未使用的导入和变量
6. FOR ALL 迁移的文件，THE Migration_Tool SHALL 保持或改善代码可读性

