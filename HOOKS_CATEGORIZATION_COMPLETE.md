# Hooks 分类重构完成报告

## 概述

成功将 27 个全局 hooks 按功能分类到 4 个目录中，提升了代码组织性和可维护性。

## 重构详情

### 1. 目录结构

创建了 4 个分类目录：

```
src/hooks/
├── data/          # 数据相关 hooks (8个)
├── network/       # 网络相关 hooks (4个)
├── system/        # 系统相关 hooks (10个)
└── ui/            # UI相关 hooks (4个)
```

### 2. 文件分类

#### Data Hooks (数据层 - 8个文件)
- `use-clash.ts` - Clash 配置和版本管理
- `use-profiles.ts` - 配置文件管理
- `use-connection-data.ts` - 连接数据
- `use-log-data.ts` - 日志数据
- `use-memory-data.ts` - 内存数据
- `use-traffic-data.ts` - 流量数据
- `use-current-proxy.ts` - 当前代理
- `use-proxy-selection.ts` - 代理选择

#### Network Hooks (网络层 - 4个文件)
- `use-network.ts` - 网络接口信息
- `use-traffic-monitor.ts` - 流量监控（Web Worker驱动）
- `use-mihomo-ws-subscription.ts` - WebSocket 订阅
- `use-proxy-delay-state.ts` - 代理延迟状态

#### System Hooks (系统层 - 10个文件)
- `use-system-state.ts` - 系统状态
- `use-system-proxy-state.ts` - 系统代理状态
- `use-verge.ts` - Verge 配置
- `use-update.ts` - 更新管理
- `use-service-installer.ts` - 服务安装
- `use-service-uninstaller.ts` - 服务卸载
- `use-connection-setting.ts` - 连接设置
- `use-clash-log.ts` - Clash 日志
- `use-listen.ts` - 事件监听
- `use-icon-cache.ts` - 图标缓存

#### UI Hooks (界面层 - 4个文件)
- `use-visibility.ts` - 可见性状态
- `use-window.ts` - 窗口控制
- `use-i18n.ts` - 国际化
- `use-editor-document.ts` - 编辑器文档

### 3. Index 文件

为每个分类目录创建了 `index.ts` 文件，统一导出该分类的所有 hooks：

```typescript
// 示例：src/hooks/data/index.ts
export { useClash, useClashInfo } from './use-clash'
export { useProfiles } from './use-profiles'
export { useConnectionData } from './use-connection-data'
// ... 其他导出
```

### 4. 导入路径更新

更新了 **64 个文件**的导入路径，从：
```typescript
import { useVerge } from '@/hooks/use-verge'
import { useProfiles } from '@/hooks/use-profiles'
```

改为：
```typescript
import { useVerge } from '@/hooks/system'
import { useProfiles } from '@/hooks/data'
```

#### 更新的文件类型分布：
- **Pages (11个)**: `_layout.tsx`, `home.tsx`, `profiles.tsx`, `proxies.tsx`, `connections.tsx`, `logs.tsx`, `rules.tsx`, `test.tsx`
- **Components (45个)**:
  - Setting 组件 (20个)
  - Proxy 组件 (8个)
  - Profile 组件 (5个)
  - Home 组件 (7个)
  - Layout 组件 (3个)
  - Test 组件 (2个)
- **Hooks (5个)**: 内部 hook 依赖更新
- **Providers (1个)**: `app-data-provider.tsx`
- **Layout Hooks (2个)**: `use-custom-theme.ts`, `use-layout-events.ts`

### 5. 内部依赖处理

正确处理了 hooks 之间的内部依赖关系：

- `use-traffic-monitor.ts` → 依赖 `@/hooks/ui` 的 `useVisibility`
- `use-system-proxy-state.ts` → 依赖同目录的 `useVerge`（相对路径）
- `use-proxy-delay-state.ts` → 依赖 `@/hooks/system` 的 `useVerge`
- `use-proxy-selection.ts` → 依赖同目录的 `useProfiles` + `@/hooks/system` 的 `useVerge`
- `use-i18n.ts` → 依赖 `@/hooks/system` 的 `useVerge`
- 数据层 hooks → 依赖 `@/hooks/network` 的 `useMihomoWsSubscription`

## 验证结果

✅ **TypeScript 类型检查通过**
```bash
pnpm exec tsc --noEmit
Exit Code: 0
```

所有导入路径正确，无类型错误。

## 改进效果

### 代码组织性
- ✅ 按功能分层，职责清晰
- ✅ 减少了 hooks 目录的文件数量（从 27 个扁平文件到 4 个分类目录）
- ✅ 更容易找到相关的 hooks

### 可维护性
- ✅ 新增 hook 时有明确的分类指导
- ✅ 通过 index 文件统一导出，便于管理
- ✅ 导入路径更简洁（`@/hooks/system` vs `@/hooks/use-verge`）

### 可扩展性
- ✅ 每个分类可以独立扩展
- ✅ 便于添加新的分类（如 `@/hooks/storage`）
- ✅ 支持按需导入，减少打包体积

## 分类原则

1. **Data**: 数据获取、状态管理、业务逻辑
2. **Network**: 网络通信、WebSocket、流量监控
3. **System**: 系统级操作、配置管理、服务控制
4. **UI**: 界面状态、窗口控制、用户交互

## 后续建议

1. **文档更新**: 在项目 README 中添加 hooks 分类说明
2. **开发规范**: 制定新 hook 的分类标准
3. **持续优化**: 根据使用情况调整分类（如某个分类文件过多时进一步细分）

## 相关文档

- [架构优化路线图](./ARCHITECTURE_OPTIMIZATION_ROADMAP.md)
- [架构分析报告](./ARCHITECTURE_ANALYSIS.md)
- [Setting 模块重构完成](./SETTING_MODULE_REFACTOR_COMPLETE.md)

---

**完成时间**: 2026-05-27  
**影响范围**: 27 个 hook 文件 + 64 个导入文件  
**测试状态**: ✅ TypeScript 类型检查通过
