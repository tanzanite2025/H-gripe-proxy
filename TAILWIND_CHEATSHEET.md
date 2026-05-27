# Tailwind CSS 迁移 - 命令速查表

## 🚀 迁移命令

```bash
# 迁移单个文件
pnpm migrate:file src/pages/unlock.tsx

# 批量迁移所有页面
pnpm migrate:all

# 验证样式
pnpm verify:styles
```

## 📦 开发命令

```bash
# 开发服务器
pnpm dev

# 类型检查
pnpm typecheck

# 构建项目
pnpm build

# 快速构建
pnpm build:fast
```

## 🔍 查找命令

```bash
# 查找所有备份文件
find src -name "*.tsx.bak"

# 删除所有备份文件
find src -name "*.tsx.bak" -delete

# 查找 MUI 导入
grep -r "@mui/material" src

# 查找 sx prop
grep -r "sx={{" src
```

## 📝 Git 命令

```bash
# 创建迁移分支
git checkout -b feature/tailwind-migration

# 查看变化
git diff src/pages/unlock.tsx

# 提交变化
git add .
git commit -m "feat: migrate to Tailwind CSS"

# 查看迁移进度
git log --oneline --grep="migrate"
```

## 🎨 常用转换

### MUI → Tailwind

```tsx
// 导入
import { Box, Button } from '@mui/material'
→ import { Box, Button } from '@/components/tailwind'

// 样式
sx={{ display: 'flex', gap: 2 }}
→ className="flex gap-2"

// 按钮
variant="contained"
→ variant="primary"

// 图标
import { Close } from '@mui/icons-material'
→ import { X } from 'lucide-react'
```

### 常用类名

```tsx
// 布局
flex flex-col items-center justify-between gap-2

// 间距
p-3 px-4 py-2 mb-4 mt-2

// 尺寸
w-full h-12 max-w-md

// 颜色
bg-primary text-white dark:bg-gray-900

// 圆角
rounded rounded-lg rounded-button

// 阴影
shadow shadow-md shadow-card
```

## 📚 文档快速链接

```bash
# 快速入门
cat TAILWIND_README.md

# 完整指南
cat TAILWIND_MIGRATION_COMPLETE.md

# 快速参考
cat TAILWIND_MIGRATION_QUICK_GUIDE.md

# 项目总结
cat TAILWIND_MIGRATION_SUMMARY.md
```

## 🛠️ 故障排除

```bash
# 清理缓存
rm -rf node_modules/.vite dist

# 重新安装依赖
pnpm install

# 重新构建
pnpm build

# 检查 Tailwind 配置
cat tailwind.config.js

# 检查 PostCSS 配置
cat postcss.config.js
```

## 📊 进度跟踪

```bash
# 查看迁移状态
cat TAILWIND_MIGRATION_STATUS.md

# 更新进度
# 编辑 TAILWIND_MIGRATION_PROGRESS.md
# 勾选已完成的页面
```

---

**提示**：将此文件保存为书签，方便随时查阅！
