#!/usr/bin/env node

/**
 * Tailwind CSS 迁移自动化脚本
 * 自动将 MUI 组件转换为 Tailwind 组件
 */

import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const projectRoot = join(__dirname, '..')

// 颜色输出
const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  blue: '\x1b[34m',
}

const log = {
  success: (msg) => console.log(`${colors.green}✓${colors.reset} ${msg}`),
  warning: (msg) => console.log(`${colors.yellow}⚠${colors.reset} ${msg}`),
  error: (msg) => console.log(`${colors.red}✗${colors.reset} ${msg}`),
  info: (msg) => console.log(`${colors.blue}ℹ${colors.reset} ${msg}`),
}

// MUI 图标到 Lucide React 的映射
const iconMap = {
  Close: 'X',
  Add: 'Plus',
  Delete: 'Trash2',
  Edit: 'Pencil',
  Settings: 'Settings',
  Check: 'Check',
  ChevronDown: 'ChevronDown',
  ChevronUp: 'ChevronUp',
  ChevronLeft: 'ChevronLeft',
  ChevronRight: 'ChevronRight',
  MoreVert: 'MoreVertical',
  MoreHoriz: 'MoreHorizontal',
  Refresh: 'RefreshCw',
  RefreshOutlined: 'RefreshCw',
  Search: 'Search',
  Info: 'Info',
  InfoOutlined: 'Info',
  Warning: 'AlertTriangle',
  WarningOutlined: 'AlertTriangle',
  Error: 'AlertCircle',
  ErrorOutlined: 'AlertCircle',
  Visibility: 'Eye',
  VisibilityOff: 'EyeOff',
  VisibilityOutlined: 'Eye',
  VisibilityOffOutlined: 'EyeOff',
  ContentCopy: 'Copy',
  ContentCopyRounded: 'Copy',
  GitHub: 'Github',
  HelpOutline: 'HelpCircle',
  HelpOutlineRounded: 'HelpCircle',
  Telegram: 'Send',
  Language: 'Globe',
  LanguageRounded: 'Globe',
  Computer: 'Monitor',
  ComputerRounded: 'Monitor',
  Troubleshoot: 'Wrench',
  TroubleshootRounded: 'Wrench',
  SwapVert: 'ArrowUpDown',
  SwapVertRounded: 'ArrowUpDown',
  FilterList: 'Filter',
  FilterListRounded: 'Filter',
  Clear: 'X',
  ClearRounded: 'X',
  Lan: 'Network',
  LanOutlined: 'Network',
  LanRounded: 'Network',
}

// sx prop 到 Tailwind className 的常见转换
const sxToTailwind = {
  "display: 'flex'": 'flex',
  "flexDirection: 'column'": 'flex-col',
  "flexDirection: 'row'": 'flex-row',
  "alignItems: 'center'": 'items-center',
  "alignItems: 'start'": 'items-start',
  "alignItems: 'end'": 'items-end',
  "justifyContent: 'center'": 'justify-center',
  "justifyContent: 'between'": 'justify-between',
  "justifyContent: 'start'": 'justify-start',
  "justifyContent: 'end'": 'justify-end',
  'gap: 1': 'gap-1',
  'gap: 2': 'gap-2',
  'gap: 3': 'gap-3',
  'gap: 4': 'gap-4',
  'p: 1': 'p-1',
  'p: 2': 'p-2',
  'p: 3': 'p-3',
  'p: 4': 'p-4',
  'px: 1': 'px-1',
  'px: 2': 'px-2',
  'px: 3': 'px-3',
  'px: 4': 'px-4',
  'py: 1': 'py-1',
  'py: 2': 'py-2',
  'py: 3': 'py-3',
  'py: 4': 'py-4',
  'pt: 1': 'pt-1',
  'pt: 1.25': 'pt-5',
  'pb: 2': 'pb-2',
  'pl: 2': 'pl-2',
  'pr: 2': 'pr-2',
  'mb: 0.5': 'mb-2',
  'mb: 4.5': 'mb-18',
  'mb: 4': 'mb-4',
  'mt: 2': 'mt-2',
}

/**
 * 转换文件内容
 */
function migrateFile(filePath) {
  if (!existsSync(filePath)) {
    log.error(`File not found: ${filePath}`)
    return false
  }

  let content = readFileSync(filePath, 'utf-8')
  let modified = false

  // 1. 替换 MUI 导入为 Tailwind 导入
  const muiImportRegex = /import\s+{([^}]+)}\s+from\s+['"]@mui\/material['"]/g
  if (muiImportRegex.test(content)) {
    content = content.replace(muiImportRegex, (match, imports) => {
      modified = true
      return `import {${imports}} from '@/components/tailwind'`
    })
    log.success('Replaced @mui/material imports')
  }

  // 2. 替换 MUI 图标导入为 Lucide React
  const iconImportRegex = /import\s+{([^}]+)}\s+from\s+['"]@mui\/icons-material['"]/g
  const iconMatches = content.match(iconImportRegex)
  if (iconMatches) {
    iconMatches.forEach((match) => {
      const iconsMatch = match.match(/{([^}]+)}/)
      if (iconsMatch) {
        const muiIcons = iconsMatch[1].split(',').map((s) => s.trim())
        const lucideIcons = muiIcons
          .map((icon) => iconMap[icon] || icon)
          .filter((icon, index, self) => self.indexOf(icon) === index) // 去重

        content = content.replace(
          match,
          `import { ${lucideIcons.join(', ')} } from 'lucide-react'`,
        )
        modified = true
        log.success(`Replaced icons: ${muiIcons.join(', ')} → ${lucideIcons.join(', ')}`)
      }
    })
  }

  // 3. 替换图标使用
  Object.entries(iconMap).forEach(([muiIcon, lucideIcon]) => {
    const iconRegex = new RegExp(`<${muiIcon}\\s*([^>]*)/>`, 'g')
    if (iconRegex.test(content)) {
      content = content.replace(iconRegex, (match, props) => {
        // 如果没有 className，添加默认尺寸
        if (!props.includes('className')) {
          return `<${lucideIcon} className="h-5 w-5" ${props}/>`
        }
        return `<${lucideIcon} ${props}/>`
      })
      modified = true
    }
  })

  // 4. 替换 Button variant
  content = content.replace(/variant="contained"/g, 'variant="primary"')
  if (content.includes('variant="primary"')) {
    modified = true
    log.success('Replaced Button variant="contained" → variant="primary"')
  }

  // 5. 简单的 sx 转换（仅处理简单情况）
  const simpleSxRegex = /sx={{([^}]+)}}/g
  const sxMatches = content.match(simpleSxRegex)
  if (sxMatches) {
    log.warning('Found sx props - manual conversion recommended')
    log.info('Use TAILWIND_MIGRATION_QUICK_GUIDE.md for reference')
  }

  // 6. 替换 Grid size prop
  content = content.replace(/size={{([^}]+)}}/g, (match, props) => {
    // 简单转换：size={{ xs: 6, sm: 4 }} → item xs={6} sm={4}
    const propsObj = props.split(',').map((p) => p.trim())
    const newProps = propsObj
      .map((p) => {
        const [key, value] = p.split(':').map((s) => s.trim())
        return `${key}={${value}}`
      })
      .join(' ')
    modified = true
    return `item ${newProps}`
  })

  if (modified) {
    // 创建备份
    const backupPath = filePath.replace(/\.tsx$/, '.tsx.bak')
    writeFileSync(backupPath, readFileSync(filePath))
    log.info(`Backup created: ${backupPath}`)

    // 写入修改后的内容
    writeFileSync(filePath, content, 'utf-8')
    log.success(`File migrated: ${filePath}`)
    return true
  }

  log.info(`No changes needed: ${filePath}`)
  return false
}

/**
 * 主函数
 */
function main() {
  const args = process.argv.slice(2)

  if (args.length === 0) {
    console.log(`
${colors.blue}Tailwind CSS Migration Tool${colors.reset}

Usage:
  node scripts/migrate-to-tailwind.mjs <file-path>

Example:
  node scripts/migrate-to-tailwind.mjs src/pages/unlock.tsx

This script will:
  1. Replace @mui/material imports with @/components/tailwind
  2. Replace @mui/icons-material with lucide-react
  3. Convert Button variant="contained" to variant="primary"
  4. Convert Grid size prop to item prop
  5. Create a backup file (.tsx.bak)

Note: Complex sx props need manual conversion.
See TAILWIND_MIGRATION_QUICK_GUIDE.md for details.
    `)
    process.exit(0)
  }

  const filePath = join(projectRoot, args[0])
  log.info(`Migrating: ${filePath}`)

  const success = migrateFile(filePath)

  if (success) {
    console.log(`
${colors.green}✓ Migration completed!${colors.reset}

Next steps:
  1. Review the changes
  2. Manually convert sx props to className
  3. Test the page functionality
  4. Delete the backup file if everything works

See TAILWIND_MIGRATION_QUICK_GUIDE.md for manual conversion guide.
    `)
  } else {
    console.log(`
${colors.yellow}⚠ No changes made${colors.reset}

The file may already be migrated or doesn't contain MUI components.
    `)
  }
}

main()
