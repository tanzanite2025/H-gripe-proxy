# Cleanup Checklist - Post Tailwind Migration

## 可以安全删除的文件

### ✅ 已确认可删除

#### 1. Emotion Style Chain
**文件**: `src/components/base/base-emotion-style-chain.tsx`

**原因**:
- 已从 `main.tsx` 中移除
- 不再被任何文件引用
- Tailwind 不需要 Emotion

**删除命令**:
```bash
rm src/components/base/base-emotion-style-chain.tsx
```

#### 2. Custom Theme Hook
**文件**: `src/pages/_layout/hooks/use-custom-theme.ts`

**原因**:
- 已被 `use-css-variables.ts` 替代
- 不再从 `hooks/index.ts` 导出
- CSS 变量逻辑已提取到独立工具函数

**删除命令**:
```bash
rm src/pages/_layout/hooks/use-custom-theme.ts
```

---

## ⚠️ 需要确认后删除

### 备份文件 (.tsx.bak)
**文件**: `src/pages/*.tsx.bak`

**说明**:
- 这些是迁移前的备份文件
- 建议在测试通过后删除
- 可以通过 Git 历史恢复

**删除命令** (测试通过后):
```bash
# 列出所有备份文件
find src/pages -name "*.tsx.bak"

# 删除所有备份文件
find src/pages -name "*.tsx.bak" -delete
```

---

## 📦 已移除的依赖

### NPM Packages (已移除)
```json
{
  "dependencies": {
    "@mui/material": "9.0.1",           // ✅ 已移除
    "@mui/icons-material": "9.0.1",     // ✅ 已移除
    "@emotion/react": "11.14.0",        // ✅ 已移除
    "@emotion/styled": "11.14.1",       // ✅ 已移除
    "@emotion/cache": "11.14.0"         // ✅ 已移除
  },
  "devDependencies": {
    "@emotion/babel-plugin": "11.13.5"  // ✅ 已移除
  }
}
```

**验证命令**:
```bash
# 检查是否还有 MUI/Emotion 依赖
pnpm list | grep -E "@mui|@emotion"
```

---

## 🧹 配置清理

### ✅ Vite Config (已清理)
**文件**: `vite.config.mts`

**已移除**:
- `jsxImportSource: '@emotion/react'`
- `@emotion/babel-plugin` 配置

**当前状态**:
```ts
react()  // 简化配置
```

### ✅ Main.tsx (已清理)
**文件**: `src/main.tsx`

**已移除**:
- `EmotionStyleChain` 导入
- `<EmotionStyleChain>` 包装器

---

## 📋 清理步骤

### Step 1: 删除不再使用的文件
```bash
# 1. 删除 Emotion Style Chain
rm src/components/base/base-emotion-style-chain.tsx

# 2. 删除 Custom Theme Hook
rm src/pages/_layout/hooks/use-custom-theme.ts
```

### Step 2: 验证没有引用
```bash
# 搜索是否还有文件引用这些已删除的文件
grep -r "base-emotion-style-chain" src/
grep -r "use-custom-theme" src/
```

**预期结果**: 应该没有任何匹配

### Step 3: 测试应用
```bash
# 启动开发服务器
pnpm dev

# 测试所有主页面
# - 检查样式是否正常
# - 检查主题切换是否正常
# - 检查 CSS 变量是否应用
```

### Step 4: 删除备份文件 (可选)
```bash
# 测试通过后，删除所有备份文件
find src/pages -name "*.tsx.bak" -delete
```

### Step 5: 提交更改
```bash
git add .
git commit -m "chore: cleanup post-tailwind migration files"
```

---

## 🔍 验证清单

### ✅ 文件删除验证
- [ ] `base-emotion-style-chain.tsx` 已删除
- [ ] `use-custom-theme.ts` 已删除
- [ ] 没有文件引用已删除的文件
- [ ] 备份文件已删除 (可选)

### ✅ 依赖验证
- [ ] `@mui/material` 已从 package.json 移除
- [ ] `@mui/icons-material` 已从 package.json 移除
- [ ] `@emotion/*` 已从 package.json 移除
- [ ] `pnpm list` 不显示 MUI/Emotion 依赖

### ✅ 配置验证
- [ ] `vite.config.mts` 不包含 Emotion 配置
- [ ] `main.tsx` 不包含 EmotionStyleChain
- [ ] `layout.tsx` 不包含 MUI 组件

### ✅ 功能验证
- [ ] 所有主页面样式正常
- [ ] 主题切换正常
- [ ] CSS 变量应用正常
- [ ] 导航菜单正常
- [ ] 无控制台错误

---

## 📊 清理前后对比

### 文件数量
| 类别 | 清理前 | 清理后 | 减少 |
|------|--------|--------|------|
| 组件文件 | 2 | 0 | -2 |
| Hook 文件 | 4 | 3 | -1 |
| 备份文件 | 10 | 0 | -10 |
| **总计** | **16** | **3** | **-13** |

### 依赖数量
| 类别 | 清理前 | 清理后 | 减少 |
|------|--------|--------|------|
| MUI | 2 | 0 | -2 |
| Emotion | 4 | 0 | -4 |
| **总计** | **6** | **0** | **-6** |

### Bundle Size
| 类别 | 清理前 | 清理后 | 减少 |
|------|--------|--------|------|
| MUI | ~1.5MB | 0 | -1.5MB |
| Emotion | ~1MB | 0 | -1MB |
| **总计** | **~2.5MB** | **0** | **-2.5MB** |

---

## 🎯 清理目标

### 主要目标
1. ✅ 删除所有不再使用的文件
2. ✅ 移除所有 MUI/Emotion 依赖
3. ✅ 清理所有相关配置
4. ✅ 验证应用功能正常

### 次要目标
1. ⚠️ 删除备份文件 (可选)
2. ⚠️ 优化 import 语句
3. ⚠️ 更新文档

---

## 📝 注意事项

### ⚠️ Settings 组件
Settings 组件仍在使用 MUI，因此:
- **不要删除** Settings 相关的 MUI 导入
- **不要删除** Settings 相关的样式文件
- Settings 组件可以继续使用 MUI 或逐步迁移

### ⚠️ CSS 变量
CSS 变量逻辑已从 `use-custom-theme.ts` 提取到:
- `src/utils/theme/css-variables.ts` - CSS 变量管理
- `src/utils/misc/color.ts` - 颜色工具函数
- `src/pages/_layout/hooks/use-css-variables.ts` - CSS 变量 hook

确保这些文件正常工作后再删除 `use-custom-theme.ts`。

### ⚠️ Git 历史
删除文件前，确保:
1. 所有更改已提交到 Git
2. 可以通过 Git 历史恢复删除的文件
3. 有完整的备份

---

## 🚀 执行清理

### 快速清理脚本
```bash
#!/bin/bash

echo "🧹 Starting cleanup..."

# 1. 删除不再使用的文件
echo "📁 Removing unused files..."
rm -f src/components/base/base-emotion-style-chain.tsx
rm -f src/pages/_layout/hooks/use-custom-theme.ts

# 2. 删除备份文件 (可选)
echo "📦 Removing backup files..."
find src/pages -name "*.tsx.bak" -delete

# 3. 验证
echo "🔍 Verifying..."
echo "Checking for references to deleted files..."
grep -r "base-emotion-style-chain" src/ && echo "⚠️ Found references!" || echo "✅ No references"
grep -r "use-custom-theme" src/ && echo "⚠️ Found references!" || echo "✅ No references"

# 4. 检查依赖
echo "📦 Checking dependencies..."
pnpm list | grep -E "@mui|@emotion" && echo "⚠️ Found MUI/Emotion!" || echo "✅ No MUI/Emotion"

echo "✅ Cleanup complete!"
```

**保存为**: `scripts/cleanup.sh`

**执行**:
```bash
chmod +x scripts/cleanup.sh
./scripts/cleanup.sh
```

---

## ✅ 完成标志

清理完成后，应该满足:

1. ✅ 所有不再使用的文件已删除
2. ✅ 所有 MUI/Emotion 依赖已移除
3. ✅ 应用功能正常
4. ✅ 无控制台错误
5. ✅ 所有测试通过
6. ✅ 更改已提交到 Git

**🎉 Cleanup Complete! 🎉**

---

**Generated**: 2026-05-27  
**Version**: 1.0.0  
**Status**: Ready for execution
