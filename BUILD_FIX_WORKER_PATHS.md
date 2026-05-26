# 构建修复：Worker 路径问题

## 问题描述

在完成架构优化后，运行 `pnpm build` 时出现构建失败，错误信息：

```
[UNRESOLVED_ENTRY] Cannot resolve entry module src/utils/yaml.worker
[UNRESOLVED_ENTRY] Cannot resolve entry module src/hooks/services/traffic-monitor-worker.ts
```

## 根本原因

在进行 Utils 分类重构时，`yaml.worker.ts` 被移动到了 `utils/misc/` 目录，但相关的导入路径没有同步更新。同时，`use-traffic-monitor.ts` 在 Hooks 分类时被移动到 `hooks/network/`，但其内部的 worker 相对路径没有更新。

## 修复详情

### 1. yaml.worker 路径修复

**文件：** `src/services/monaco.ts`

**修复前：**
```typescript
import('@/utils/yaml.worker?worker'),
```

**修复后：**
```typescript
import('@/utils/misc/yaml.worker?worker'),
```

**原因：** `yaml.worker.ts` 现在位于 `utils/misc/` 目录

### 2. traffic-monitor-worker 路径修复

**文件：** `src/hooks/network/use-traffic-monitor.ts`

**修复前：**
```typescript
this.worker = new Worker(
  new URL('../services/traffic-monitor-worker.ts', import.meta.url),
  { type: 'module' },
)
```

**修复后：**
```typescript
this.worker = new Worker(
  new URL('../../services/traffic-monitor-worker.ts', import.meta.url),
  { type: 'module' },
)
```

**原因：** `use-traffic-monitor.ts` 从 `hooks/` 移动到 `hooks/network/`，相对路径需要多一层 `../`

## 验证结果

✅ **TypeScript 类型检查通过**
```bash
pnpm exec tsc --noEmit
Exit Code: 0
```

✅ **Web 构建成功**
```bash
pnpm run web:build
✓ 14543 modules transformed.
✓ built in 5.58s
Exit Code: 0
```

✅ **完整构建成功**
- 版本 0.0.3 构建完成
- 生成文件：
  - `Clash Verge Optimized_0.0.3_x64-setup.exe`
  - `Clash Verge Optimized_0.0.3_x64-setup.exe.sig`（用于自动更新）

## 经验教训

### 1. Worker 文件的特殊性

Worker 文件使用 `new URL(..., import.meta.url)` 或 `?worker` 后缀导入，这些路径在构建时会被特殊处理。移动文件时需要特别注意更新这些路径。

### 2. 相对路径的脆弱性

相对路径（`../`）在文件移动时容易出错。建议：
- 优先使用绝对路径（`@/` 别名）
- 如果必须使用相对路径，在移动文件后立即验证

### 3. 构建验证的重要性

架构重构后应该：
1. 运行 TypeScript 类型检查（`tsc --noEmit`）
2. 运行完整构建（`pnpm build`）
3. 运行测试（如果有）

仅通过类型检查不足以发现所有问题，因为 Worker 路径是在构建时解析的。

## 预防措施

### 1. 搜索所有引用

在移动文件前，使用 grep 搜索所有引用：
```bash
# 搜索文件名
grep -r "yaml.worker" src/
grep -r "traffic-monitor-worker" src/
```

### 2. 检查特殊导入

注意以下特殊导入模式：
- `?worker` 后缀
- `new URL(..., import.meta.url)`
- `?raw` 后缀
- 动态 `import()`

### 3. 分阶段验证

大规模重构时，建议分阶段进行：
1. 移动文件
2. 更新导入
3. 运行类型检查
4. 运行构建
5. 提交代码

每个阶段都验证通过后再进行下一步。

## 相关文档

- [Utils 分类完成报告](./UTILS_CATEGORIZATION_COMPLETE.md)
- [Hooks 分类完成报告](./HOOKS_CATEGORIZATION_COMPLETE.md)
- [架构优化路线图](./ARCHITECTURE_OPTIMIZATION_ROADMAP.md)

---

**修复时间：** 2026-05-27  
**影响文件：** 2 个文件  
**测试状态：** ✅ 构建成功
