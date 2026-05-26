# 多路复用与混沌动态混淆 - 下一步行动计划

## ✅ 已完成

### 阶段 1: 代码重构 (完成)

**目标**: 重构 `proxy-groups.tsx` 为模块化结构

**成果**:
- ✅ 将 1100+ 行组件拆分为 10 个模块
- ✅ 主组件减少到 250 行
- ✅ 职责清晰分离（UI、逻辑、工具）
- ✅ 通过 TypeScript 类型检查
- ✅ 保持所有原有功能

**详细报告**: 查看 `PROXY_GROUPS_REFACTOR_COMPLETE.md`

---

## 🎯 下一步选择

现在有三个方向可以选择：

### 选项 A: 增强代理节点显示 (推荐) 🌟

**优先级**: 🟡 中
**预计时间**: 1-2 小时
**难度**: ⭐⭐ (简单)

**目标**: 显示更详细的多路复用信息

**具体任务**:
1. 增强 `proxy-item.tsx` 和 `proxy-item-mini.tsx`
2. 显示 SMUX 协议类型（smux/yamux/h2mux）
3. 显示 Mieru 和 Sudoku 的多路复用状态
4. 添加 Tooltip 显示完整配置

**改进示例**:
```typescript
// 当前显示
<TypeBox>SMUX</TypeBox>

// 改进后显示
<Tooltip title="协议: yamux, 最大连接: 4">
  <TypeBox>SMUX (yamux)</TypeBox>
</Tooltip>

// Mieru 多路复用
{proxy.type === 'mieru' && proxy.multiplexing && (
  <TypeBox>MUX ({proxy.multiplexing})</TypeBox>
)}

// Sudoku HTTP Mask
{proxy.type === 'sudoku' && proxy.httpmask?.multiplex && (
  <TypeBox>HTTP-MUX ({proxy.httpmask.multiplex})</TypeBox>
)}
```

**为什么推荐**:
- 用户可以直观看到多路复用状态
- 为后续配置界面做准备
- 改动小，风险低
- 立即提升用户体验

---

### 选项 B: 创建多路复用配置界面

**优先级**: 🟡 中
**预计时间**: 3-4 小时
**难度**: ⭐⭐⭐ (中等)

**目标**: 提供多路复用配置界面

**具体任务**:
1. 创建 SMUX 配置组件
2. 创建 Mieru 多路复用配置组件
3. 创建 Sudoku HTTP Mask 配置组件
4. 集成到代理链或代理编辑界面

**新增文件**:
```
src/components/proxy/multiplexing/
├── index.tsx                        # 多路复用配置主组件
├── smux-config.tsx                  # SMUX 配置
├── mieru-multiplex-config.tsx       # Mieru 多路复用配置
├── sudoku-multiplex-config.tsx      # Sudoku 多路复用配置
└── multiplexing-stats.tsx           # 多路复用统计
```

**配置界面示例**:
```typescript
// SMUX 配置
<SmuxConfig
  protocol="yamux"  // smux | yamux | h2mux
  maxConnections={4}
  minStreams={1}
  maxStreams={0}
  padding={false}
  onConfigChange={handleConfigChange}
/>

// Mieru 配置
<MieruMultiplexConfig
  level="MULTIPLEXING_MIDDLE"  // OFF | LOW | MIDDLE | HIGH
  onLevelChange={handleLevelChange}
/>
```

**为什么重要**:
- 用户可以自定义多路复用参数
- 提高代理性能和稳定性
- 为高级用户提供更多控制

---

### 选项 C: 实现混沌动态混淆 (可选)

**优先级**: 🟢 低
**预计时间**: 6-8 小时
**难度**: ⭐⭐⭐⭐⭐ (困难)

**目标**: 实现流量混淆功能

**具体任务**:
1. 创建混淆服务（流量混淆、协议混淆）
2. 创建混淆配置界面
3. 集成到代理链
4. 性能测试和优化

**新增文件**:
```
src/services/obfuscation/
├── index.ts                         # 混淆管理器
├── traffic-obfuscation.ts           # 流量混淆
├── protocol-obfuscation.ts          # 协议混淆
└── obfuscation-strategies.ts        # 混淆策略

src/components/proxy/obfuscation/
├── index.tsx                        # 混淆配置主组件
├── obfuscation-level-selector.tsx   # 混淆级别选择器
├── obfuscation-strategy-config.tsx  # 混淆策略配置
└── obfuscation-stats.tsx            # 混淆效果统计
```

**混淆功能**:
1. **流量特征混淆**
   - 随机化包大小
   - 随机化时序间隔
   - 添加随机填充数据

2. **协议混淆**
   - HTTP/HTTPS 伪装
   - TLS 指纹随机化
   - 自定义协议头

3. **动态策略**
   - 根据网络环境自动调整
   - 多种混淆模式切换
   - 智能检测和规避

**为什么是可选**:
- 实现复杂，需要大量时间
- 可能需要后端（Rust）支持
- 对大多数用户来说不是必需功能
- 可以作为高级功能后续添加

---

## 💡 我的建议

### 推荐路径：A → B → C

**第一步: 增强代理节点显示** (选项 A)
- 快速提升用户体验
- 为后续功能做准备
- 风险低，收益高

**第二步: 创建多路复用配置界面** (选项 B)
- 提供实用的配置功能
- 满足高级用户需求
- 完善多路复用功能

**第三步: 实现混沌动态混淆** (选项 C，可选)
- 作为高级功能
- 根据用户反馈决定是否实现
- 可以分阶段实现

---

## 📋 详细实施计划

### 阶段 2: 增强代理节点显示 (推荐先做)

#### 任务 2.1: 增强 proxy-item.tsx

**文件**: `src/components/proxy/proxy-item.tsx`

**改动**:
```typescript
// 1. 添加 SMUX 详细信息显示
{showType && proxy.smux && (
  <Tooltip title={getSmuxTooltip(proxy)}>
    <TypeBox>
      SMUX {proxy.smux?.protocol && `(${proxy.smux.protocol})`}
    </TypeBox>
  </Tooltip>
)}

// 2. 添加 Mieru 多路复用显示
{showType && proxy.type === 'mieru' && proxy.multiplexing && (
  <Tooltip title={`Mieru 多路复用: ${proxy.multiplexing}`}>
    <TypeBox>MUX ({proxy.multiplexing.replace('MULTIPLEXING_', '')})</TypeBox>
  </Tooltip>
)}

// 3. 添加 Sudoku HTTP Mask 显示
{showType && proxy.type === 'sudoku' && proxy.httpmask?.multiplex && (
  <Tooltip title={`HTTP Mask 多路复用: ${proxy.httpmask.multiplex}`}>
    <TypeBox>HTTP-MUX ({proxy.httpmask.multiplex})</TypeBox>
  </Tooltip>
)}
```

**新增工具函数**:
```typescript
// src/components/proxy/utils/multiplexing-helpers.ts
export function getSmuxTooltip(proxy: IProxyItem): string {
  if (!proxy.smux) return ''
  
  const parts = []
  if (proxy.smux.protocol) parts.push(`协议: ${proxy.smux.protocol}`)
  if (proxy.smux['max-connections']) parts.push(`最大连接: ${proxy.smux['max-connections']}`)
  if (proxy.smux['min-streams']) parts.push(`最小流: ${proxy.smux['min-streams']}`)
  
  return parts.join(', ')
}
```

#### 任务 2.2: 同步更新 proxy-item-mini.tsx

**文件**: `src/components/proxy/proxy-item-mini.tsx`

**改动**: 与 `proxy-item.tsx` 相同的逻辑

#### 任务 2.3: 验证

1. 运行类型检查：`pnpm run typecheck`
2. 测试显示效果
3. 检查 Tooltip 是否正常工作

---

### 阶段 3: 创建多路复用配置界面 (可选)

#### 任务 3.1: 创建 SMUX 配置组件

**文件**: `src/components/proxy/multiplexing/smux-config.tsx`

**功能**:
- 协议选择（smux/yamux/h2mux）
- 连接数配置
- 流数配置
- Brutal 优化配置

#### 任务 3.2: 创建 Mieru 配置组件

**文件**: `src/components/proxy/multiplexing/mieru-multiplex-config.tsx`

**功能**:
- 级别选择（OFF/LOW/MIDDLE/HIGH）
- 流量模式配置

#### 任务 3.3: 创建 Sudoku 配置组件

**文件**: `src/components/proxy/multiplexing/sudoku-multiplex-config.tsx`

**功能**:
- 模式选择（off/auto/on）
- HTTP mask 参数配置

#### 任务 3.4: 集成到代理链

**文件**: `src/components/proxy/proxy-chain.tsx`

**改动**:
- 为每个节点添加多路复用配置按钮
- 显示多路复用状态
- 保存配置到代理链

---

## 🤔 需要你决定

请告诉我你想先做哪个：

1. **选项 A**: 增强代理节点显示（推荐，1-2小时）
2. **选项 B**: 创建多路复用配置界面（3-4小时）
3. **选项 C**: 实现混沌动态混淆（6-8小时，可选）
4. **其他**: 你有其他想法吗？

或者你想：
- 先看看现有的多路复用实现效果？
- 创建一个 Spec 文件来详细规划？
- 直接开始实现某个功能？

**请告诉我你的选择，我会立即开始实施！** 🚀
