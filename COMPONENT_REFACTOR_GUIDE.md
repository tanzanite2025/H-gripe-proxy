# 组件重构实战指南

## 🎯 快速开始

这是一份实战指南，帮助你快速重构大型组件。

---

## 📋 重构步骤模板

### 步骤 1：分析组件职责

```bash
# 1. 打开组件文件
# 2. 列出所有功能
# 3. 按职责分组
```

**示例：** `enhanced-canvas-traffic-graph.tsx`

```
职责列表：
1. 数据处理
   - 流量数据采样
   - 数据格式化
   - 数据缓存

2. 渲染逻辑
   - Canvas 绘图
   - 动画控制
   - 性能优化

3. 用户交互
   - 鼠标悬停
   - 点击事件
   - 缩放控制

4. 主题适配
   - 颜色切换
   - 样式更新
```

### 步骤 2：制定拆分方案

```
拆分方案：
1. hooks/use-traffic-graph-data.ts     → 数据处理
2. hooks/use-canvas-renderer.ts        → 渲染逻辑
3. hooks/use-graph-interaction.ts      → 用户交互
4. utils/graph-calculator.ts           → 计算工具
5. enhanced-canvas-traffic-graph.tsx   → 主组件（组合）
```

### 步骤 3：创建目录结构

```bash
# 为大型组件创建子目录
mkdir src/components/home/enhanced-canvas-traffic-graph
cd src/components/home/enhanced-canvas-traffic-graph

# 创建子目录
mkdir hooks
mkdir utils
```

### 步骤 4：提取 Hooks

**原则：**
- 一个 hook 只做一件事
- hook 名称清晰表达职责
- hook 之间可以组合

**示例：**

```typescript
// hooks/use-traffic-graph-data.ts
export const useTrafficGraphData = () => {
  const [data, setData] = useState<TrafficData[]>([])
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    // 数据采样逻辑
  }, [])

  return { data, loading }
}

// hooks/use-canvas-renderer.ts
export const useCanvasRenderer = (
  canvasRef: RefObject<HTMLCanvasElement>,
  data: TrafficData[],
  theme: Theme
) => {
  useEffect(() => {
    if (!canvasRef.current) return
    const ctx = canvasRef.current.getContext('2d')
    // 绘图逻辑
  }, [data, theme])
}

// hooks/use-graph-interaction.ts
export const useGraphInteraction = (canvasRef: RefObject<HTMLCanvasElement>) => {
  const [hoveredPoint, setHoveredPoint] = useState<Point | null>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const handleMouseMove = (e: MouseEvent) => {
      // 交互逻辑
    }

    canvas.addEventListener('mousemove', handleMouseMove)
    return () => canvas.removeEventListener('mousemove', handleMouseMove)
  }, [])

  return { hoveredPoint }
}
```

### 步骤 5：提取工具函数

**原则：**
- 纯函数，无副作用
- 易于测试
- 可以复用

**示例：**

```typescript
// utils/graph-calculator.ts

/**
 * 计算图表坐标点
 */
export const calculateGraphPoints = (
  data: TrafficData[],
  width: number,
  height: number
): Point[] => {
  // 计算逻辑
  return points
}

/**
 * 计算 Y 轴刻度
 */
export const calculateYAxisScale = (
  maxValue: number,
  height: number
): number[] => {
  // 计算逻辑
  return scales
}

/**
 * 格式化流量值
 */
export const formatTrafficValue = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}
```

### 步骤 6：重构主组件

**原则：**
- 主组件只负责组合
- 逻辑都在 hooks 和工具函数中
- 保持简洁（< 200 行）

**示例：**

```typescript
// index.tsx
import { useRef } from 'react'
import { useTheme } from '@mui/material'
import { useTrafficGraphData } from './hooks/use-traffic-graph-data'
import { useCanvasRenderer } from './hooks/use-canvas-renderer'
import { useGraphInteraction } from './hooks/use-graph-interaction'

export const EnhancedCanvasTrafficGraph = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const theme = useTheme()

  // 数据处理
  const { data, loading } = useTrafficGraphData()

  // 渲染逻辑
  useCanvasRenderer(canvasRef, data, theme)

  // 用户交互
  const { hoveredPoint } = useGraphInteraction(canvasRef)

  if (loading) {
    return <div>Loading...</div>
  }

  return (
    <div>
      <canvas ref={canvasRef} width={800} height={400} />
      {hoveredPoint && (
        <div>
          {hoveredPoint.x}, {hoveredPoint.y}
        </div>
      )}
    </div>
  )
}
```

### 步骤 7：更新导入路径

```typescript
// 其他文件中的导入
// 之前
import { EnhancedCanvasTrafficGraph } from '@/components/home/enhanced-canvas-traffic-graph'

// 之后（如果创建了子目录）
import { EnhancedCanvasTrafficGraph } from '@/components/home/enhanced-canvas-traffic-graph'
// 路径不变，因为有 index.tsx
```

### 步骤 8：测试验证

```bash
# 1. 类型检查
pnpm run typecheck

# 2. 构建测试
pnpm run build

# 3. 开发环境测试
pnpm run dev
# 手动测试所有功能

# 4. 性能测试
# 使用 React DevTools Profiler
```

---

## 🎨 重构模式

### 模式 1：表单组件重构

**适用于：** 复杂表单组件（如 `dns-config.tsx`）

```typescript
// 拆分前（1000+ 行）
const DnsConfig = () => {
  // 表单状态
  // 验证逻辑
  // 提交逻辑
  // UI 渲染
}

// 拆分后
// 1. hooks/use-dns-form.ts
export const useDnsForm = () => {
  const [values, setValues] = useState({})
  const [errors, setErrors] = useState({})

  const validate = () => { /* 验证逻辑 */ }
  const submit = () => { /* 提交逻辑 */ }

  return { values, errors, validate, submit }
}

// 2. components/dns-server-list.tsx
export const DnsServerList = ({ servers, onChange }) => {
  // 只负责展示服务器列表
}

// 3. components/dns-rule-editor.tsx
export const DnsRuleEditor = ({ rules, onChange }) => {
  // 只负责编辑规则
}

// 4. index.tsx
export const DnsConfig = () => {
  const form = useDnsForm()
  return (
    <>
      <DnsServerList servers={form.values.servers} onChange={form.handleChange} />
      <DnsRuleEditor rules={form.values.rules} onChange={form.handleChange} />
    </>
  )
}
```

### 模式 2：列表组件重构

**适用于：** 复杂列表组件（如 `groups-editor-viewer.tsx`）

```typescript
// 拆分前（1000+ 行）
const GroupsEditor = () => {
  // 列表数据
  // 搜索过滤
  // 拖拽排序
  // 编辑逻辑
  // UI 渲染
}

// 拆分后
// 1. hooks/use-group-list.ts
export const useGroupList = () => {
  const [groups, setGroups] = useState([])
  const [filteredGroups, setFilteredGroups] = useState([])

  const search = (keyword: string) => { /* 搜索逻辑 */ }
  const reorder = (from: number, to: number) => { /* 排序逻辑 */ }

  return { groups, filteredGroups, search, reorder }
}

// 2. components/group-list.tsx
export const GroupList = ({ groups, onSelect }) => {
  // 只负责展示列表
}

// 3. components/group-search.tsx
export const GroupSearch = ({ onSearch }) => {
  // 只负责搜索
}

// 4. components/group-form.tsx
export const GroupForm = ({ group, onSave }) => {
  // 只负责编辑
}

// 5. index.tsx
export const GroupsEditor = () => {
  const { groups, filteredGroups, search, reorder } = useGroupList()
  const [selectedGroup, setSelectedGroup] = useState(null)

  return (
    <>
      <GroupSearch onSearch={search} />
      <GroupList groups={filteredGroups} onSelect={setSelectedGroup} />
      {selectedGroup && <GroupForm group={selectedGroup} onSave={handleSave} />}
    </>
  )
}
```

### 模式 3：卡片组件重构

**适用于：** 信息展示卡片（如 `current-proxy-card.tsx`）

```typescript
// 拆分前（1000+ 行）
const CurrentProxyCard = () => {
  // 代理数据
  // 延迟测试
  // 代理切换
  // UI 渲染
}

// 拆分后
// 1. hooks/use-current-proxy.ts
export const useCurrentProxy = () => {
  const [proxy, setProxy] = useState(null)
  const [delay, setDelay] = useState(0)

  const testDelay = async () => { /* 测试逻辑 */ }
  const switchProxy = async (name: string) => { /* 切换逻辑 */ }

  return { proxy, delay, testDelay, switchProxy }
}

// 2. components/proxy-info-display.tsx
export const ProxyInfoDisplay = ({ proxy }) => {
  // 只负责展示代理信息
}

// 3. components/proxy-actions.tsx
export const ProxyActions = ({ onTest, onSwitch }) => {
  // 只负责操作按钮
}

// 4. index.tsx
export const CurrentProxyCard = () => {
  const { proxy, delay, testDelay, switchProxy } = useCurrentProxy()

  return (
    <Card>
      <ProxyInfoDisplay proxy={proxy} delay={delay} />
      <ProxyActions onTest={testDelay} onSwitch={switchProxy} />
    </Card>
  )
}
```

---

## ⚠️ 常见陷阱

### 陷阱 1：过度拆分

```typescript
// ❌ 错误：拆分过细
const ProfileCard = () => {
  return (
    <>
      <ProfileCardHeader />
      <ProfileCardTitle />
      <ProfileCardSubtitle />
      <ProfileCardBody />
      <ProfileCardFooter />
      <ProfileCardActions />
    </>
  )
}

// ✅ 正确：合理拆分
const ProfileCard = () => {
  return (
    <>
      <ProfileHeader />  // 包含 title 和 subtitle
      <ProfileBody />
      <ProfileFooter />  // 包含 actions
    </>
  )
}
```

### 陷阱 2：Hook 依赖混乱

```typescript
// ❌ 错误：Hook 之间相互依赖
const useA = () => {
  const b = useB()  // A 依赖 B
  return { a: b.value }
}

const useB = () => {
  const a = useA()  // B 依赖 A（循环依赖）
  return { value: a.value }
}

// ✅ 正确：单向依赖
const useData = () => {
  // 基础数据
  return { data }
}

const useProcessedData = () => {
  const { data } = useData()  // 只依赖基础数据
  return { processed: process(data) }
}
```

### 陷阱 3：Props 传递过深

```typescript
// ❌ 错误：Props 传递 3 层以上
<A prop1={value}>
  <B prop1={value}>
    <C prop1={value}>
      <D prop1={value} />
    </C>
  </B>
</A>

// ✅ 正确：使用 Context
const ValueContext = createContext(value)

<ValueContext.Provider value={value}>
  <A>
    <B>
      <C>
        <D />  // 直接从 Context 获取
      </C>
    </B>
  </A>
</ValueContext.Provider>
```

### 陷阱 4：忘记更新导入

```typescript
// ❌ 错误：移动文件后忘记更新导入
// 文件已移动到 components/profile/profile-card/index.tsx
// 但其他文件还在用旧路径
import { ProfileCard } from '@/components/profile/profile-card'  // 404

// ✅ 正确：使用 IDE 的重构功能
// 或者手动更新所有导入
import { ProfileCard } from '@/components/profile/profile-card'  // 正确
```

---

## 🔧 实用工具

### 1. 查找大型组件

```bash
# Windows PowerShell
Get-ChildItem -Path "src\components" -Recurse -Filter "*.tsx" | 
  ForEach-Object { 
    [PSCustomObject]@{ 
      File = $_.FullName.Replace((Get-Location).Path + '\', ''); 
      Lines = (Get-Content $_.FullName).Count 
    } 
  } | 
  Where-Object { $_.Lines -gt 500 } | 
  Sort-Object Lines -Descending | 
  Format-Table -AutoSize
```

### 2. 查找组件依赖

```bash
# 查找某个组件被哪些文件引用
grep -r "import.*ProfileCard" src/
```

### 3. 统计组件大小

```bash
# 统计某个目录下所有组件的总行数
Get-ChildItem -Path "src\components\profile" -Recurse -Filter "*.tsx" | 
  ForEach-Object { (Get-Content $_.FullName).Count } | 
  Measure-Object -Sum
```

---

## 📝 重构检查清单

### 开始前

- [ ] 阅读组件代码，理解所有功能
- [ ] 列出组件的所有职责
- [ ] 制定拆分方案
- [ ] 创建新的目录结构

### 重构中

- [ ] 提取 hooks（数据、逻辑、交互）
- [ ] 提取工具函数（计算、格式化、验证）
- [ ] 拆分子组件（展示、表单、列表）
- [ ] 重构主组件（组合）
- [ ] 更新导入路径

### 完成后

- [ ] 运行 `pnpm run typecheck`
- [ ] 运行 `pnpm run build`
- [ ] 手动测试所有功能
- [ ] 检查性能（React DevTools）
- [ ] 更新文档
- [ ] 提交代码

---

## 🎯 重构目标

### 组件大小

- 🎯 主组件：< 200 行
- 🎯 子组件：< 100 行
- 🎯 Hook：< 150 行
- 🎯 工具函数：< 50 行

### 代码质量

- 🎯 每个文件职责单一
- 🎯 函数命名清晰
- 🎯 类型定义完整
- 🎯 注释适当

### 可维护性

- 🎯 易于理解
- 🎯 易于测试
- 🎯 易于扩展
- 🎯 易于复用

---

## 📚 参考资源

1. **COMPONENT_RESPONSIBILITY_ANALYSIS.md** - 组件职责分析
2. **ARCHITECTURE_OPTIMIZATION_ROADMAP.md** - 架构优化路线图
3. **React 官方文档** - https://react.dev/
4. **Hooks 最佳实践** - https://react.dev/reference/react

---

## 💡 快速参考

### 何时拆分组件？

- ✅ 组件超过 500 行
- ✅ 组件有多个职责
- ✅ 组件难以理解
- ✅ 组件难以测试
- ✅ 组件有重复代码

### 如何命名？

- ✅ 组件：`ProfileCard`, `ProfileList`, `ProfileForm`
- ✅ Hook：`useProfileData`, `useProfileOperations`
- ✅ 工具：`validateProfile`, `formatProfile`

### 如何组织文件？

```
component-name/
├── index.tsx           # 主组件
├── hooks/              # Hooks
│   ├── use-data.ts
│   └── use-operations.ts
├── components/         # 子组件
│   ├── header.tsx
│   └── body.tsx
└── utils/              # 工具函数
    └── helpers.ts
```

---

**文档创建时间：** 2026-05-27  
**文档版本：** v1.0  
**适用范围：** 所有组件重构工作
