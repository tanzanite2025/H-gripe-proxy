# 项目架构分析报告

## 📊 当前架构概览

### 目录结构

```
src/
├── assets/          # 静态资源
│   ├── fonts/       # 字体文件
│   ├── image/       # 图片资源
│   └── styles/      # 全局样式（SCSS）
├── components/      # 组件目录 ⚠️
│   ├── base/        # 基础组件（17个文件）
│   ├── connection/  # 连接相关（4个文件）
│   ├── home/        # 首页组件（11个文件）
│   ├── layout/      # 布局组件（7个文件）
│   ├── log/         # 日志组件（1个文件）
│   ├── profile/     # 配置文件组件（15个文件）
│   ├── proxy/       # 代理组件（12个文件）
│   ├── rule/        # 规则组件（2个文件）
│   ├── setting/     # 设置组件（4个文件 + 29个mods）⚠️
│   ├── shared/      # 共享组件（2个文件）
│   ├── test/        # 测试组件（3个文件）
│   └── uds/         # UDS组件（1个文件）
├── hooks/           # 自定义Hooks（27个文件）⚠️
├── locales/         # 国际化（13种语言 × 10个文件）
├── pages/           # 页面组件（10个页面）
│   └── _layout/     # 布局相关
│       ├── hooks/   # 布局Hooks（5个文件）⚠️
│       └── utils/   # 布局工具（3个文件）⚠️
├── polyfills/       # 兼容性补丁（3个文件）
├── providers/       # Context提供者（5个文件）
├── services/        # 服务层（14个文件）⚠️
├── types/           # 类型定义（5个文件）
└── utils/           # 工具函数（18个文件 + uri-parser）⚠️
    └── uri-parser/  # URI解析器（20个文件）⚠️
```

---

## 🚨 发现的主要问题

### 1. 组件目录过度分散 ⚠️⚠️⚠️

**问题：**
- `components/` 下有 **12 个子目录**
- 按页面功能分类（home, profile, proxy, rule, setting, connection, log, test）
- 按组件类型分类（base, layout, shared, uds）
- **分类标准不统一，混乱**

**具体问题：**
```
components/
├── base/           # 按类型分类
├── shared/         # 按类型分类
├── layout/         # 按类型分类
├── uds/            # 按类型分类
├── home/           # 按页面分类
├── profile/        # 按页面分类
├── proxy/          # 按页面分类
├── setting/        # 按页面分类
├── connection/     # 按页面分类
├── log/            # 按页面分类
├── rule/           # 按页面分类
└── test/           # 按页面分类
```

**混乱点：**
- `layout/` 既是组件类型，又是页面功能
- `shared/` 只有 2 个文件，存在意义不明确
- `uds/` 只有 1 个文件（icons.tsx），应该合并
- `test/` 是测试页面还是测试组件？不清楚

---

### 2. setting 组件过度细分 ⚠️⚠️⚠️

**问题：**
```
components/setting/
├── setting-clash.tsx
├── setting-system.tsx
├── setting-verge-advanced.tsx
├── setting-verge-basic.tsx
└── mods/  # 29 个小组件！⚠️
    ├── auto-backup-settings.tsx
    ├── backup-config-viewer.tsx
    ├── backup-history-viewer.tsx
    ├── backup-viewer.tsx
    ├── backup-webdav-dialog.tsx
    ├── clash-core-viewer.tsx
    ├── clash-port-viewer.tsx
    ├── config-viewer.tsx
    ├── controller-viewer.tsx
    ├── dns-viewer.tsx
    ├── external-controller-cors.tsx
    ├── guard-state.tsx
    ├── hotkey-input.tsx
    ├── hotkey-viewer.tsx
    ├── layout-viewer.tsx
    ├── lite-mode-viewer.tsx
    ├── misc-viewer.tsx
    ├── network-interface-viewer.tsx
    ├── password-input.tsx
    ├── setting-comp.tsx
    ├── stack-mode-switch.tsx
    ├── sysproxy-viewer.tsx
    ├── theme-mode-switch.tsx
    ├── theme-viewer.tsx
    ├── tun-viewer.tsx
    ├── tunnels-viewer.tsx
    ├── update-viewer.tsx
    ├── web-ui-item.tsx
    └── web-ui-viewer.tsx
```

**问题分析：**
- **29 个小组件**，过度拆分
- 命名不统一：`-viewer`, `-input`, `-switch`, `-dialog`, `-settings`
- 很多组件可能只有几十行代码
- 维护成本高，查找困难

---

### 3. Hooks 过多且分散 ⚠️⚠️

**问题：**
- 全局 `hooks/` 目录：**27 个文件**
- `pages/_layout/hooks/`：**5 个文件**
- `components/proxy/`：**4 个 hook 文件**

**具体问题：**
```
hooks/  # 27 个全局 hooks
├── use-clash-log.ts
├── use-clash.ts
├── use-connection-data.ts
├── use-connection-setting.ts
├── use-current-proxy.ts
├── use-editor-document.ts
├── use-i18n.ts
├── use-icon-cache.ts
├── use-listen.ts
├── use-log-data.ts
├── use-memory-data.ts
├── use-mihomo-ws-subscription.ts
├── use-network.ts
├── use-profiles.ts
├── use-proxy-delay-state.ts
├── use-proxy-selection.ts
├── use-service-installer.ts
├── use-service-uninstaller.ts
├── use-system-proxy-state.ts
├── use-system-state.ts
├── use-traffic-data.ts
├── use-traffic-monitor.ts
├── use-update.ts
├── use-verge.ts
├── use-visibility.ts
├── use-window.ts
└── use-xxx.ts

pages/_layout/hooks/  # 5 个布局 hooks
├── use-custom-theme.ts
├── use-layout-events.ts
├── use-loading-overlay.ts
└── use-nav-menu-order.ts

components/proxy/  # 4 个代理 hooks
├── use-filter-sort.ts
├── use-head-state.ts
├── use-render-list.ts
└── use-window-width.ts
```

**混乱点：**
- 没有按功能分类
- 布局相关的 hooks 为什么在 `pages/_layout/hooks/`？
- 代理相关的 hooks 为什么在 `components/proxy/`？
- 全局 hooks 太多，难以查找

---

### 4. utils 工具函数混乱 ⚠️⚠️

**问题：**
```
utils/
├── data-validator.ts
├── debounce.ts
├── debug.ts
├── disable-webview-shortcuts.ts
├── get-system.ts
├── ignore-case.ts
├── is-async-function.ts
├── network.ts
├── noop.ts
├── parse-hotkey.ts
├── parse-traffic.ts
├── search-matcher.ts
├── traffic-diagnostics.ts
├── traffic-sampler.ts
├── truncate-str.ts
├── yaml.worker.ts
└── uri-parser/  # 20 个文件！⚠️
    ├── anytls.ts
    ├── helpers.ts
    ├── http.ts
    ├── hysteria.ts
    ├── hysteria2.ts
    ├── index.ts
    ├── mieru.ts
    ├── snell.ts
    ├── socks.ts
    ├── ss.ts
    ├── ssh.ts
    ├── ssr.ts
    ├── sudoku.ts
    ├── transport.ts
    ├── trojan-go.ts
    ├── trojan.ts
    ├── tuic.ts
    ├── vless.ts
    ├── vmess.ts
    └── wireguard.ts
```

**问题分析：**
- 工具函数没有分类
- `uri-parser/` 有 20 个文件，但没有进一步分类
- 功能相关的工具没有放在一起（如 traffic-* 相关）

---

### 5. services 服务层职责不清 ⚠️

**问题：**
```
services/
├── api.ts                      # API 调用
├── cmds.ts                     # 命令调用
├── config.ts                   # 配置管理
├── delay.ts                    # 延迟测试
├── i18n.ts                     # 国际化
├── monaco.ts                   # Monaco 编辑器
├── notice-service.ts           # 通知服务
├── preload.ts                  # 预加载
├── query-client.ts             # React Query 客户端
├── states.ts                   # 状态管理
├── traffic-monitor-worker.ts  # 流量监控 Worker
├── update.ts                   # 更新服务
└── webdav-status.ts            # WebDAV 状态
```

**问题分析：**
- 职责混乱：有 API、配置、状态、工具、服务
- `states.ts` 应该是状态管理，为什么在 services？
- `i18n.ts` 应该是工具，为什么在 services？
- `monaco.ts` 应该是工具，为什么在 services？

---

### 6. pages/_layout 结构混乱 ⚠️

**问题：**
```
pages/
├── _layout.tsx          # 布局组件
├── _layout/             # 布局相关目录
│   ├── hooks/           # 布局 hooks
│   └── utils/           # 布局工具
├── _routers.tsx         # 路由配置
├── _theme.tsx           # 主题配置
└── xxx.tsx              # 页面组件
```

**问题分析：**
- `_layout.tsx` 和 `_layout/` 目录并存，混乱
- 为什么布局有自己的 hooks 和 utils？
- `_routers.tsx` 和 `_theme.tsx` 为什么在 pages 下？

---

## 📈 架构问题统计

| 问题类型 | 严重程度 | 影响范围 |
|---------|---------|---------|
| 组件分类混乱 | 🔴 高 | 全局 |
| setting 过度细分 | 🔴 高 | 设置模块 |
| Hooks 分散 | 🟡 中 | 全局 |
| utils 无分类 | 🟡 中 | 工具层 |
| services 职责不清 | 🟡 中 | 服务层 |
| pages/_layout 混乱 | 🟢 低 | 布局层 |

---

## 🎯 推荐的架构优化方案

### 方案 A：按功能模块重组（推荐）⭐

```
src/
├── features/              # 功能模块（按业务领域）
│   ├── home/              # 首页模块
│   │   ├── components/    # 首页组件
│   │   ├── hooks/         # 首页 hooks
│   │   └── utils/         # 首页工具
│   ├── proxy/             # 代理模块
│   │   ├── components/
│   │   ├── hooks/
│   │   └── utils/
│   ├── profile/           # 配置文件模块
│   │   ├── components/
│   │   ├── hooks/
│   │   └── utils/
│   ├── connection/        # 连接模块
│   │   ├── components/
│   │   └── hooks/
│   ├── rule/              # 规则模块
│   │   ├── components/
│   │   └── hooks/
│   ├── log/               # 日志模块
│   │   ├── components/
│   │   └── hooks/
│   ├── setting/           # 设置模块
│   │   ├── components/
│   │   │   ├── clash/     # Clash 设置
│   │   │   ├── system/    # 系统设置
│   │   │   ├── verge/     # Verge 设置
│   │   │   └── shared/    # 共享组件
│   │   └── hooks/
│   └── test/              # 测试模块
│       ├── components/
│       └── hooks/
├── shared/                # 共享资源
│   ├── components/        # 共享组件
│   │   ├── ui/            # UI 组件（原 base/）
│   │   ├── layout/        # 布局组件
│   │   └── icons/         # 图标组件（原 uds/）
│   ├── hooks/             # 共享 hooks
│   │   ├── data/          # 数据相关
│   │   ├── ui/            # UI 相关
│   │   └── system/        # 系统相关
│   ├── utils/             # 共享工具
│   │   ├── format/        # 格式化工具
│   │   ├── parser/        # 解析工具（uri-parser）
│   │   ├── network/       # 网络工具
│   │   └── validation/    # 验证工具
│   └── services/          # 共享服务
│       ├── api/           # API 服务
│       ├── config/        # 配置服务
│       └── notification/  # 通知服务
├── core/                  # 核心层
│   ├── providers/         # Context 提供者
│   ├── router/            # 路由配置
│   ├── theme/             # 主题配置
│   └── i18n/              # 国际化
├── assets/                # 静态资源
│   ├── fonts/
│   ├── images/
│   └── styles/
└── types/                 # 类型定义
```

**优势：**
- ✅ 按业务领域组织，清晰明确
- ✅ 每个模块自包含（组件、hooks、utils）
- ✅ 共享资源统一管理
- ✅ 易于维护和扩展

---

### 方案 B：保持现有结构，局部优化（保守）

**优化重点：**

1. **合并小目录**
   ```
   components/
   ├── ui/              # 合并 base + shared + uds
   ├── layout/          # 保持
   ├── home/            # 保持
   ├── proxy/           # 保持
   ├── profile/         # 保持
   ├── connection/      # 保持
   ├── rule/            # 保持
   ├── log/             # 保持
   ├── setting/         # 保持，但优化 mods
   └── test/            # 保持
   ```

2. **优化 setting/mods**
   ```
   setting/
   ├── setting-clash.tsx
   ├── setting-system.tsx
   ├── setting-verge-advanced.tsx
   ├── setting-verge-basic.tsx
   └── components/      # 重命名 mods
       ├── backup/      # 合并 backup 相关（5个文件）
       ├── clash/       # 合并 clash 相关（3个文件）
       ├── network/     # 合并 network 相关（3个文件）
       ├── theme/       # 合并 theme 相关（2个文件）
       ├── hotkey/      # 合并 hotkey 相关（2个文件）
       └── misc/        # 其他（14个文件）
   ```

3. **分类 hooks**
   ```
   hooks/
   ├── data/            # 数据相关（8个）
   │   ├── use-clash.ts
   │   ├── use-profiles.ts
   │   ├── use-connection-data.ts
   │   └── ...
   ├── ui/              # UI 相关（5个）
   │   ├── use-visibility.ts
   │   ├── use-window.ts
   │   └── ...
   ├── network/         # 网络相关（4个）
   │   ├── use-network.ts
   │   ├── use-traffic-data.ts
   │   └── ...
   └── system/          # 系统相关（10个）
       ├── use-system-state.ts
       ├── use-update.ts
       └── ...
   ```

4. **分类 utils**
   ```
   utils/
   ├── format/          # 格式化
   │   ├── parse-traffic.ts
   │   ├── truncate-str.ts
   │   └── ...
   ├── parser/          # 解析器
   │   └── uri-parser/
   ├── network/         # 网络
   │   ├── network.ts
   │   └── ...
   ├── validation/      # 验证
   │   └── data-validator.ts
   └── misc/            # 其他
       ├── debounce.ts
       ├── noop.ts
       └── ...
   ```

---

## 🎯 优先级建议

### 第一优先级（立即优化）🔴

1. **优化 setting/mods**
   - 影响：设置模块
   - 难度：中
   - 收益：高
   - 时间：2-3 小时

2. **合并小目录**
   - 合并 `components/shared/`（2个文件）
   - 合并 `components/uds/`（1个文件）
   - 影响：组件层
   - 难度：低
   - 收益：中
   - 时间：30 分钟

### 第二优先级（1周内）🟡

3. **分类 hooks**
   - 影响：全局
   - 难度：中
   - 收益：高
   - 时间：3-4 小时

4. **分类 utils**
   - 影响：工具层
   - 难度：中
   - 收益：中
   - 时间：2-3 小时

### 第三优先级（长期）🟢

5. **重组为功能模块**（方案 A）
   - 影响：全局
   - 难度：高
   - 收益：非常高
   - 时间：1-2 周

---

## 📝 下一步行动

### 建议从哪个模块开始？

**推荐：setting 模块** ⭐

**原因：**
1. 问题最明显（29 个小文件）
2. 影响范围可控（只影响设置页面）
3. 优化收益明显
4. 风险较低

**具体步骤：**
1. 分析 29 个 mods 文件的功能
2. 按功能分组（backup, clash, network, theme, etc.）
3. 合并相关文件
4. 重命名 `mods/` 为 `components/`
5. 测试设置页面功能

---

## 🎯 总结

### 当前架构的主要问题

1. **组件分类混乱**（按类型 vs 按页面）
2. **setting 过度细分**（29 个小文件）
3. **Hooks 分散**（3 个不同位置）
4. **utils 无分类**（18 个文件 + 20 个 uri-parser）
5. **services 职责不清**（混合了 API、配置、状态、工具）

### 推荐方案

- **短期**：方案 B（局部优化，从 setting 开始）
- **长期**：方案 A（按功能模块重组）

### 预期收益

- ✅ 代码更易查找
- ✅ 模块职责更清晰
- ✅ 维护成本降低
- ✅ 新人上手更快
- ✅ 扩展性更好

---

**文档创建时间：** 2026-05-27 05:35  
**分析范围：** src/ 目录完整架构  
**文档版本：** v1.0
