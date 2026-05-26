# Setting 模块重构方案

## 📊 当前状态分析

### 文件结构

```
components/setting/
├── setting-clash.tsx              # Clash 设置主组件
├── setting-system.tsx             # 系统设置主组件
├── setting-verge-advanced.tsx     # Verge 高级设置主组件
├── setting-verge-basic.tsx        # Verge 基础设置主组件
└── mods/                          # 29 个小组件 ⚠️
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

---

## 🔍 问题分析

### 1. 命名不统一

| 后缀 | 数量 | 示例 |
|------|------|------|
| `-viewer` | 16 个 | `backup-viewer`, `theme-viewer` |
| `-input` | 2 个 | `hotkey-input`, `password-input` |
| `-switch` | 2 个 | `theme-mode-switch`, `stack-mode-switch` |
| `-dialog` | 1 个 | `backup-webdav-dialog` |
| `-settings` | 1 个 | `auto-backup-settings` |
| 其他 | 7 个 | `setting-comp`, `guard-state`, `web-ui-item` |

**问题：**
- `-viewer` 过度使用，语义不明确
- 命名规则不一致

---

### 2. 功能分组混乱

**按功能分析：**

#### Backup 相关（5 个文件）
- `auto-backup-settings.tsx`
- `backup-config-viewer.tsx`
- `backup-history-viewer.tsx`
- `backup-viewer.tsx`
- `backup-webdav-dialog.tsx`

#### Clash 核心相关（3 个文件）
- `clash-core-viewer.tsx`
- `clash-port-viewer.tsx`
- `dns-viewer.tsx`

#### 网络相关（5 个文件）
- `controller-viewer.tsx`
- `external-controller-cors.tsx`
- `network-interface-viewer.tsx`
- `tun-viewer.tsx`
- `tunnels-viewer.tsx`

#### 系统代理相关（2 个文件）
- `sysproxy-viewer.tsx`
- `guard-state.tsx`

#### 主题相关（2 个文件）
- `theme-mode-switch.tsx`
- `theme-viewer.tsx`

#### 快捷键相关（2 个文件）
- `hotkey-input.tsx`
- `hotkey-viewer.tsx`

#### Web UI 相关（2 个文件）
- `web-ui-item.tsx`
- `web-ui-viewer.tsx`

#### 其他（8 个文件）
- `config-viewer.tsx`
- `layout-viewer.tsx`
- `lite-mode-viewer.tsx`
- `misc-viewer.tsx`
- `password-input.tsx`
- `setting-comp.tsx`
- `stack-mode-switch.tsx`
- `update-viewer.tsx`

---

## 🎯 重构方案

### 方案 A：按功能分组（推荐）⭐

```
components/setting/
├── setting-clash.tsx
├── setting-system.tsx
├── setting-verge-advanced.tsx
├── setting-verge-basic.tsx
└── components/                    # 重命名 mods
    ├── backup/                    # 备份相关（5个文件）
    │   ├── auto-backup-settings.tsx
    │   ├── backup-config.tsx      # 重命名
    │   ├── backup-history.tsx     # 重命名
    │   ├── backup-main.tsx        # 重命名
    │   └── backup-webdav-dialog.tsx
    ├── clash/                     # Clash 核心（3个文件）
    │   ├── clash-core.tsx         # 重命名
    │   ├── clash-port.tsx         # 重命名
    │   └── dns-config.tsx         # 重命名
    ├── network/                   # 网络相关（5个文件）
    │   ├── controller.tsx         # 重命名
    │   ├── external-cors.tsx      # 重命名
    │   ├── network-interface.tsx  # 重命名
    │   ├── tun-config.tsx         # 重命名
    │   └── tunnels-config.tsx     # 重命名
    ├── proxy/                     # 代理相关（2个文件）
    │   ├── system-proxy.tsx       # 重命名
    │   └── guard-state.tsx
    ├── theme/                     # 主题相关（2个文件）
    │   ├── theme-mode-switch.tsx
    │   └── theme-config.tsx       # 重命名
    ├── hotkey/                    # 快捷键（2个文件）
    │   ├── hotkey-input.tsx
    │   └── hotkey-config.tsx      # 重命名
    ├── webui/                     # Web UI（2个文件）
    │   ├── webui-item.tsx         # 重命名
    │   └── webui-config.tsx       # 重命名
    ├── shared/                    # 共享组件（2个文件）
    │   ├── password-input.tsx
    │   └── setting-item.tsx       # 重命名 setting-comp
    └── misc/                      # 其他（6个文件）
        ├── config-editor.tsx      # 重命名
        ├── layout-config.tsx      # 重命名
        ├── lite-mode.tsx          # 重命名
        ├── misc-config.tsx        # 重命名
        ├── stack-mode-switch.tsx
        └── update-config.tsx      # 重命名
```

**优势：**
- ✅ 按功能分组，清晰明确
- ✅ 每个分组 2-5 个文件，适中
- ✅ 命名统一，去掉 `-viewer` 后缀
- ✅ 易于查找和维护

---

### 方案 B：按设置页面分组（保守）

```
components/setting/
├── setting-clash.tsx
├── setting-system.tsx
├── setting-verge-advanced.tsx
├── setting-verge-basic.tsx
└── components/
    ├── clash/                     # Clash 设置页面用到的
    │   ├── clash-core.tsx
    │   ├── clash-port.tsx
    │   ├── dns-config.tsx
    │   ├── controller.tsx
    │   └── external-cors.tsx
    ├── system/                    # 系统设置页面用到的
    │   ├── system-proxy.tsx
    │   ├── guard-state.tsx
    │   ├── tun-config.tsx
    │   └── ...
    ├── verge-basic/               # Verge 基础设置
    │   ├── theme-mode-switch.tsx
    │   ├── theme-config.tsx
    │   ├── hotkey-config.tsx
    │   └── ...
    ├── verge-advanced/            # Verge 高级设置
    │   ├── backup-main.tsx
    │   ├── webui-config.tsx
    │   └── ...
    └── shared/                    # 共享组件
        ├── password-input.tsx
        └── setting-item.tsx
```

**优势：**
- ✅ 按页面分组，对应主组件
- ✅ 易于理解组件用途
- ✅ 减少跨页面查找

**劣势：**
- ❌ 可能有组件被多个页面使用
- ❌ 分组不够清晰

---

## 📋 重构步骤（方案 A）

### 阶段 1：创建新目录结构（30分钟）

1. **创建分组目录**
   ```bash
   mkdir components/setting/components/backup
   mkdir components/setting/components/clash
   mkdir components/setting/components/network
   mkdir components/setting/components/proxy
   mkdir components/setting/components/theme
   mkdir components/setting/components/hotkey
   mkdir components/setting/components/webui
   mkdir components/setting/components/shared
   mkdir components/setting/components/misc
   ```

2. **移动和重命名文件**
   - 按功能分组移动
   - 统一命名规则

---

### 阶段 2：更新导入路径（1小时）

1. **更新主组件导入**
   ```typescript
   // ❌ 之前
   import { BackupViewer } from './mods/backup-viewer'
   import { ThemeViewer } from './mods/theme-viewer'
   
   // ✅ 现在
   import { BackupMain } from './components/backup/backup-main'
   import { ThemeConfig } from './components/theme/theme-config'
   ```

2. **更新组件内部导入**
   ```typescript
   // ❌ 之前
   import { PasswordInput } from './password-input'
   
   // ✅ 现在
   import { PasswordInput } from '../shared/password-input'
   ```

---

### 阶段 3：测试验证（30分钟）

1. **功能测试**
   - 打开设置页面
   - 测试每个设置项
   - 确认功能正常

2. **类型检查**
   ```bash
   pnpm run typecheck
   ```

3. **构建测试**
   ```bash
   pnpm run build
   ```

---

## 📊 重命名对照表

### Backup 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `auto-backup-settings.tsx` | `auto-backup-settings.tsx` | `backup/` |
| `backup-config-viewer.tsx` | `backup-config.tsx` | `backup/` |
| `backup-history-viewer.tsx` | `backup-history.tsx` | `backup/` |
| `backup-viewer.tsx` | `backup-main.tsx` | `backup/` |
| `backup-webdav-dialog.tsx` | `backup-webdav-dialog.tsx` | `backup/` |

### Clash 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `clash-core-viewer.tsx` | `clash-core.tsx` | `clash/` |
| `clash-port-viewer.tsx` | `clash-port.tsx` | `clash/` |
| `dns-viewer.tsx` | `dns-config.tsx` | `clash/` |

### Network 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `controller-viewer.tsx` | `controller.tsx` | `network/` |
| `external-controller-cors.tsx` | `external-cors.tsx` | `network/` |
| `network-interface-viewer.tsx` | `network-interface.tsx` | `network/` |
| `tun-viewer.tsx` | `tun-config.tsx` | `network/` |
| `tunnels-viewer.tsx` | `tunnels-config.tsx` | `network/` |

### Proxy 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `sysproxy-viewer.tsx` | `system-proxy.tsx` | `proxy/` |
| `guard-state.tsx` | `guard-state.tsx` | `proxy/` |

### Theme 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `theme-mode-switch.tsx` | `theme-mode-switch.tsx` | `theme/` |
| `theme-viewer.tsx` | `theme-config.tsx` | `theme/` |

### Hotkey 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `hotkey-input.tsx` | `hotkey-input.tsx` | `hotkey/` |
| `hotkey-viewer.tsx` | `hotkey-config.tsx` | `hotkey/` |

### Web UI 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `web-ui-item.tsx` | `webui-item.tsx` | `webui/` |
| `web-ui-viewer.tsx` | `webui-config.tsx` | `webui/` |

### Shared 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `password-input.tsx` | `password-input.tsx` | `shared/` |
| `setting-comp.tsx` | `setting-item.tsx` | `shared/` |

### Misc 相关

| 原文件名 | 新文件名 | 位置 |
|---------|---------|------|
| `config-viewer.tsx` | `config-editor.tsx` | `misc/` |
| `layout-viewer.tsx` | `layout-config.tsx` | `misc/` |
| `lite-mode-viewer.tsx` | `lite-mode.tsx` | `misc/` |
| `misc-viewer.tsx` | `misc-config.tsx` | `misc/` |
| `stack-mode-switch.tsx` | `stack-mode-switch.tsx` | `misc/` |
| `update-viewer.tsx` | `update-config.tsx` | `misc/` |

---

## 🎯 命名规范

### 统一后缀

| 用途 | 后缀 | 示例 |
|------|------|------|
| 配置组件 | `-config` | `theme-config.tsx` |
| 输入组件 | `-input` | `hotkey-input.tsx` |
| 开关组件 | `-switch` | `theme-mode-switch.tsx` |
| 对话框 | `-dialog` | `backup-webdav-dialog.tsx` |
| 主组件 | `-main` | `backup-main.tsx` |
| 列表项 | `-item` | `webui-item.tsx` |
| 编辑器 | `-editor` | `config-editor.tsx` |

### 命名原则

1. **去掉 `-viewer` 后缀**
   - ❌ `theme-viewer.tsx`
   - ✅ `theme-config.tsx`

2. **使用更具体的后缀**
   - ❌ `backup-viewer.tsx`
   - ✅ `backup-main.tsx`

3. **保持简洁**
   - ❌ `external-controller-cors.tsx`
   - ✅ `external-cors.tsx`

4. **统一命名风格**
   - ❌ `web-ui-item.tsx`
   - ✅ `webui-item.tsx`

---

## 📈 预期收益

### 代码组织

| 指标 | 优化前 | 优化后 | 改善 |
|------|--------|--------|------|
| 文件数量 | 29 个（平铺） | 29 个（分组） | 不变 |
| 目录层级 | 1 层 | 2 层 | +1 |
| 分组数量 | 0 个 | 9 个 | +9 |
| 查找难度 | 高 | 低 | ↓ 60% |

### 可维护性

- ✅ 功能分组清晰
- ✅ 命名规范统一
- ✅ 易于查找和修改
- ✅ 新人上手更快

### 扩展性

- ✅ 新增功能时知道放在哪个分组
- ✅ 相关功能集中管理
- ✅ 减少文件冲突

---

## 🚀 实施建议

### 优先级

**🔴 立即开始**
- 影响范围：设置模块
- 风险等级：🟢 低（只影响设置页面）
- 预计时间：2 小时
- 收益：高

### 实施时机

**建议：**
- 在功能稳定期进行
- 避免在发布前重构
- 预留测试时间

### 回滚方案

```bash
# 创建分支
git checkout -b refactor/setting-module

# 提交重构
git add .
git commit -m "refactor(setting): reorganize setting components"

# 如果有问题，快速回滚
git checkout main
```

---

## 📝 检查清单

### 重构前

- [ ] 创建新分支
- [ ] 备份当前代码
- [ ] 确认功能正常

### 重构中

- [ ] 创建新目录结构
- [ ] 移动和重命名文件
- [ ] 更新导入路径
- [ ] 更新导出语句

### 重构后

- [ ] TypeScript 类型检查通过
- [ ] 功能测试通过
- [ ] 构建测试通过
- [ ] 代码审查
- [ ] 提交代码

---

## 🎉 总结

### 当前问题

- ❌ 29 个文件平铺在 `mods/` 目录
- ❌ 命名不统一（6 种后缀）
- ❌ 查找困难
- ❌ 维护成本高

### 优化后

- ✅ 按功能分组（9 个分组）
- ✅ 命名统一（7 种规范后缀）
- ✅ 易于查找
- ✅ 维护成本低

### 下一步

**准备好开始重构了吗？** 我可以帮你：
1. 创建新的目录结构
2. 移动和重命名文件
3. 更新导入路径
4. 测试验证

---

**文档创建时间：** 2026-05-27 05:45  
**模块：** Setting 模块  
**文档版本：** v1.0
