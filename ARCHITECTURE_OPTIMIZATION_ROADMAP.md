# 架构优化路线图

## 📊 当前进度

### ✅ 已完成

1. **!important 优化** - 完成度 100%
   - 移除 93 处 !important（78%）
   - 保留 27 处核心设计规范
   - 耗时：85 分钟

2. **Setting 模块重构** - 完成度 100%
   - 29 个文件按功能分组（9 个分组）
   - 统一命名规范
   - 耗时：30 分钟

3. **合并小目录** - 完成度 100%
   - 合并 `components/shared/` 和 `components/uds/` 到 `components/ui/`
   - 减少目录数量 17%
   - 耗时：15 分钟

4. **Hooks 分类** - 完成度 100% ✨
   - 27 个 hooks 按功能分类到 4 个目录
   - 更新 64 个文件的导入路径
   - 创建 4 个 index 文件统一导出
   - 耗时：45 分钟

5. **Utils 分类** - 完成度 100% ✨
   - 36 个工具文件按功能分类到 5 个目录
   - 更新 50+ 个文件的导入路径
   - 创建 5 个 index 文件统一导出
   - 重组 URI 解析器（20个文件）
   - 耗时：60 分钟

---

## 🎯 下一步优化计划

### 阶段 1：合并小目录（立即开始）⚡

**目标：** 合并 `components/shared/` 和 `components/uds/`

**当前状态：**
```
components/
├── shared/          # 只有 2 个文件 ⚠️
│   ├── proxy-control-switches.tsx
│   └── traffic-error-boundary.tsx
└── uds/             # 只有 1 个文件 ⚠️
    └── icons.tsx
```

**优化方案：**
```
components/
└── ui/              # 合并后的 UI 组件目录
    ├── icons/
    │   └── icons.tsx  # 原 uds/icons.tsx
    ├── proxy-control-switches.tsx  # 原 shared/
    └── traffic-error-boundary.tsx  # 原 shared/
```

**预计时间：** 15 分钟  
**风险等级：** 🟢 低

---

### 阶段 2：分类 Hooks（1周内）📦 ✅ 已完成

**目标：** 将 27 个全局 hooks 按功能分类

**完成状态：**
```
hooks/
├── data/            # 数据相关（8个）✅
│   ├── use-clash.ts
│   ├── use-profiles.ts
│   ├── use-connection-data.ts
│   ├── use-log-data.ts
│   ├── use-memory-data.ts
│   ├── use-traffic-data.ts
│   ├── use-current-proxy.ts
│   ├── use-proxy-selection.ts
│   └── index.ts
├── network/         # 网络相关（4个）✅
│   ├── use-network.ts
│   ├── use-traffic-monitor.ts
│   ├── use-mihomo-ws-subscription.ts
│   ├── use-proxy-delay-state.ts
│   └── index.ts
├── system/          # 系统相关（10个）✅
│   ├── use-system-state.ts
│   ├── use-system-proxy-state.ts
│   ├── use-verge.ts
│   ├── use-update.ts
│   ├── use-service-installer.ts
│   ├── use-service-uninstaller.ts
│   ├── use-connection-setting.ts
│   ├── use-clash-log.ts
│   ├── use-listen.ts
│   ├── use-icon-cache.ts
│   └── index.ts
└── ui/              # UI 相关（4个）✅
    ├── use-visibility.ts
    ├── use-window.ts
    ├── use-i18n.ts
    ├── use-editor-document.ts
    └── index.ts
```

**完成情况：**
- ✅ 创建 4 个分类目录
- ✅ 移动 27 个 hook 文件
- ✅ 创建 4 个 index.ts 文件
- ✅ 更新 64 个文件的导入路径
- ✅ TypeScript 类型检查通过

**实际时间：** 45 分钟  
**风险等级：** 🟢 低（已完成）

**详细文档：** [HOOKS_CATEGORIZATION_COMPLETE.md](./HOOKS_CATEGORIZATION_COMPLETE.md)

---

### 阶段 3：分类 Utils（1周内）🔧 ✅ 已完成

**目标：** 将 18 个工具函数 + 20 个 uri-parser 按功能分类

**完成状态：**
```
utils/
├── format/          # 格式化工具（3个）✅
│   ├── parse-traffic.ts
│   ├── truncate-str.ts
│   ├── parse-hotkey.ts
│   └── index.ts
├── parser/          # 解析器（20个）✅
│   └── uri/         # URI 解析器
│       ├── ss.ts, ssr.ts, vmess.ts, vless.ts
│       ├── trojan.ts, trojan-go.ts
│       ├── hysteria.ts, hysteria2.ts
│       ├── tuic.ts, wireguard.ts
│       ├── socks.ts, http.ts, ssh.ts
│       ├── snell.ts, mieru.ts, sudoku.ts, anytls.ts
│       ├── helpers.ts, transport.ts
│       └── index.ts
├── network/         # 网络工具（3个）✅
│   ├── network.ts
│   ├── traffic-diagnostics.ts
│   ├── traffic-sampler.ts
│   └── index.ts
├── validation/      # 验证工具（2个）✅
│   ├── data-validator.ts
│   ├── search-matcher.ts
│   └── index.ts
└── misc/            # 其他工具（8个）✅
    ├── debounce.ts, noop.ts, debug.ts
    ├── get-system.ts, ignore-case.ts
    ├── is-async-function.ts
    ├── disable-webview-shortcuts.ts
    ├── yaml.worker.ts
    └── index.ts
```

**完成情况：**
- ✅ 创建 5 个分类目录
- ✅ 移动 36 个工具文件
- ✅ 创建 5 个 index.ts 文件
- ✅ 更新 50+ 个文件的导入路径
- ✅ 处理默认导出和命名导出兼容性
- ✅ TypeScript 类型检查通过

**实际时间：** 60 分钟  
**风险等级：** 🟢 低（已完成）

**详细文档：** [UTILS_CATEGORIZATION_COMPLETE.md](./UTILS_CATEGORIZATION_COMPLETE.md)

---

### 阶段 4：优化 pages/_layout（2周内）📄

**目标：** 整理 pages/_layout 结构

**当前状态：**
```
pages/
├── _layout.tsx          # 布局组件
├── _layout/             # 布局相关目录 ⚠️
│   ├── hooks/           # 5 个 hooks
│   └── utils/           # 3 个 utils
├── _routers.tsx         # 路由配置
├── _theme.tsx           # 主题配置
└── xxx.tsx              # 页面组件
```

**优化方案：**
```
pages/
├── _layout/
│   ├── layout.tsx       # 重命名 _layout.tsx
│   ├── hooks/
│   │   ├── use-custom-theme.ts
│   │   ├── use-layout-events.ts
│   │   ├── use-loading-overlay.ts
│   │   └── use-nav-menu-order.ts
│   └── utils/
│       ├── initial-loading-overlay.ts
│       └── notification-handlers.ts
├── _core/               # 核心配置
│   ├── router.tsx       # 重命名 _routers.tsx
│   └── theme.tsx        # 重命名 _theme.tsx
└── xxx.tsx              # 页面组件
```

**预计时间：** 1 小时  
**风险等级：** 🟢 低

---

### 阶段 5：重组为功能模块（长期）🏗️

**目标：** 采用 features/ 架构

**当前状态：**
```
src/
├── components/  # 按页面 + 类型混合分类 ⚠️
├── hooks/       # 全局 hooks ⚠️
├── pages/       # 页面组件
└── services/    # 服务层
```

**优化方案：**
```
src/
├── features/              # 功能模块
│   ├── home/
│   │   ├── components/
│   │   ├── hooks/
│   │   └── utils/
│   ├── proxy/
│   │   ├── components/
│   │   ├── hooks/
│   │   └── utils/
│   ├── profile/
│   │   ├── components/
│   │   ├── hooks/
│   │   └── utils/
│   ├── connection/
│   ├── rule/
│   ├── log/
│   ├── setting/  # 已优化 ✅
│   └── test/
├── shared/                # 共享资源
│   ├── components/
│   │   ├── ui/
│   │   ├── layout/
│   │   └── icons/
│   ├── hooks/
│   ├── utils/
│   └── services/
├── core/                  # 核心层
│   ├── providers/
│   ├── router/
│   ├── theme/
│   └── i18n/
└── assets/
```

**预计时间：** 1-2 周  
**风险等级：** 🔴 高

---

## 📅 时间表

### 第 1 周

- [x] **Day 1**: !important 优化（已完成）
- [x] **Day 2**: Setting 模块重构（已完成）
- [x] **Day 3**: 合并小目录（已完成）
- [x] **Day 4-5**: 分类 Hooks（已完成）✨

### 第 2 周

- [x] **Day 1-2**: 分类 Utils（已完成）✨
- [ ] **Day 3**: 优化 pages/_layout
- [ ] **Day 4-5**: 测试和调整

### 第 3-4 周

- [ ] **Week 3**: 规划功能模块重组
- [ ] **Week 4**: 开始功能模块重组

---

## 🎯 优先级矩阵

| 任务 | 影响范围 | 难度 | 收益 | 优先级 | 状态 |
|------|---------|------|------|--------|------|
| 合并小目录 | 小 | 低 | 中 | 🔴 高 | ✅ 完成 |
| 分类 Hooks | 中 | 中 | 高 | 🟡 中 | ✅ 完成 |
| 分类 Utils | 中 | 中 | 中 | 🟡 中 | ✅ 完成 |
| 优化 _layout | 小 | 低 | 低 | 🟢 低 | ⏭️ 下一步 |
| 功能模块重组 | 大 | 高 | 非常高 | 🟢 低（长期）| ⏭️ 待定 |

---

## 💡 实施建议

### 1. 渐进式优化

**原则：**
- 一次优化一个模块
- 每次优化后立即测试
- 避免大规模改动

**示例：**
```
✅ Setting 模块（已完成）
  ↓
✅ 合并小目录（已完成）
  ↓
✅ 分类 Hooks（已完成）
  ↓
✅ 分类 Utils（已完成）
  ↓
⏭️ 优化 pages/_layout（下一步）
  ↓
⏭️ 功能模块重组
```

### 2. 测试驱动

**每次优化后：**
```bash
# 1. 类型检查
pnpm run typecheck

# 2. 构建测试
pnpm run build

# 3. 功能测试
pnpm run dev
# 手动测试相关功能
```

### 3. 版本控制

**分支策略：**
```bash
# 每个优化任务创建独立分支
git checkout -b refactor/merge-small-dirs
git checkout -b refactor/categorize-hooks
git checkout -b refactor/categorize-utils

# 完成后合并到主分支
git checkout main
git merge refactor/xxx
```

---

## 📊 预期收益

### 短期收益（1个月）

| 指标 | 当前 | 目标 | 改善 |
|------|------|------|------|
| 目录层级 | 混乱 | 清晰 | ↑ 80% |
| 查找效率 | 低 | 高 | ↑ 60% |
| 命名一致性 | 差 | 好 | ↑ 70% |
| 新人上手 | 困难 | 容易 | ↑ 50% |

### 长期收益（3个月）

| 指标 | 当前 | 目标 | 改善 |
|------|------|------|------|
| 代码组织 | 混乱 | 清晰 | ↑ 90% |
| 可维护性 | 差 | 优秀 | ↑ 80% |
| 扩展性 | 差 | 优秀 | ↑ 85% |
| 团队协作 | 困难 | 顺畅 | ↑ 70% |

---

## 🎓 经验总结

### Setting 模块重构的成功经验

1. **充分规划**
   - ✅ 提前分析文件关系
   - ✅ 制定详细的重命名对照表
   - ✅ 明确分组策略

2. **渐进式执行**
   - ✅ 先创建目录结构
   - ✅ 再移动文件
   - ✅ 最后更新导入

3. **及时验证**
   - ✅ 每个阶段后运行类型检查
   - ✅ 发现问题立即修复

### 可复用的模式

**文件分组原则：**
1. 按功能分组（backup, clash, network, etc.）
2. 每组 2-5 个文件
3. 相关功能放在一起

**命名规范：**
1. 统一后缀（`-config`, `-input`, `-switch`, `-dialog`）
2. 去掉过度使用的 `-viewer`
3. 保持简洁

**导入更新：**
1. 先更新主组件
2. 再更新其他引用
3. 最后更新内部依赖

---

## 📝 下一步行动

### 立即开始（今天）

**任务：** 优化 pages/_layout

**步骤：**
1. 重命名 `_layout.tsx` 为 `_layout/layout.tsx`
2. 创建 `_core/` 目录
3. 移动 `_routers.tsx` 到 `_core/router.tsx`
4. 移动 `_theme.tsx` 到 `_core/theme.tsx`
5. 更新导入路径
6. 运行类型检查

**预计时间：** 1 小时

---

## 🎯 成功标准

### 短期目标（1个月）

- ✅ 完成 Setting 模块重构
- ✅ 合并小目录
- ✅ 分类 Hooks
- ✅ 分类 Utils
- ⏭️ 优化 pages/_layout

### 长期目标（3个月）

- ⏭️ 重组为功能模块
- ⏭️ 建立架构规范文档
- ⏭️ 团队培训
- ⏭️ 自动化检测工具

---

## 📚 相关文档

1. **ARCHITECTURE_ANALYSIS.md** - 架构分析报告
2. **SETTING_MODULE_REFACTOR_PLAN.md** - Setting 重构方案
3. **SETTING_MODULE_REFACTOR_COMPLETE.md** - Setting 重构完成报告
4. **MERGE_SMALL_DIRS_COMPLETE.md** - 合并小目录完成报告
5. **HOOKS_CATEGORIZATION_COMPLETE.md** - Hooks 分类完成报告
6. **UTILS_CATEGORIZATION_COMPLETE.md** - Utils 分类完成报告
7. **ARCHITECTURE_OPTIMIZATION_ROADMAP.md** - 架构优化路线图（本文档）

---

**文档创建时间：** 2026-05-27 06:10  
**最后更新时间：** 2026-05-27 08:00  
**当前阶段：** Utils 分类完成  
**下一步：** 优化 pages/_layout  
**文档版本：** v1.3
