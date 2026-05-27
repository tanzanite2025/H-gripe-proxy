#!/usr/bin/env node

/**
 * 批量迁移脚本
 * 自动迁移所有页面文件
 */

import { execSync } from 'node:child_process'
import { readdirSync, statSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const projectRoot = join(__dirname, '..')

const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
}

// 需要迁移的文件列表（按优先级排序）
const filesToMigrate = [
  // 优先级 1：简单页面
  'src/pages/unlock.tsx',
  
  // 优先级 2：中等复杂度
  'src/pages/settings.tsx',
  'src/pages/rules.tsx',
  'src/pages/logs.tsx',
  
  // 优先级 3：复杂页面
  'src/pages/home.tsx',
  'src/pages/connections.tsx',
  'src/pages/profiles.tsx',
  'src/pages/proxies.tsx',
  'src/pages/advanced.tsx',
]

function migrateFile(filePath) {
  console.log(`\n${colors.cyan}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${colors.reset}`)
  console.log(`${colors.blue}Migrating:${colors.reset} ${filePath}`)
  console.log(`${colors.cyan}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${colors.reset}`)
  
  try {
    execSync(`node scripts/migrate-to-tailwind.mjs ${filePath}`, {
      cwd: projectRoot,
      stdio: 'inherit',
    })
    return true
  } catch (error) {
    console.error(`${colors.yellow}⚠ Failed to migrate: ${filePath}${colors.reset}`)
    return false
  }
}

function main() {
  console.log(`
${colors.green}╔═══════════════════════════════════════════════════════╗
║                                                       ║
║   Tailwind CSS Batch Migration Tool                  ║
║                                                       ║
╚═══════════════════════════════════════════════════════╝${colors.reset}

This will migrate ${filesToMigrate.length} files to Tailwind CSS.

Files to migrate:
${filesToMigrate.map((f, i) => `  ${i + 1}. ${f}`).join('\n')}

Press Ctrl+C to cancel, or wait 3 seconds to continue...
  `)

  // 等待 3 秒
  setTimeout(() => {
    console.log(`\n${colors.green}Starting migration...${colors.reset}\n`)

    let successCount = 0
    let failCount = 0

    filesToMigrate.forEach((file, index) => {
      console.log(`\n[${index + 1}/${filesToMigrate.length}]`)
      const success = migrateFile(file)
      if (success) {
        successCount++
      } else {
        failCount++
      }
    })

    console.log(`
${colors.green}╔═══════════════════════════════════════════════════════╗
║                                                       ║
║   Migration Summary                                   ║
║                                                       ║
╚═══════════════════════════════════════════════════════╝${colors.reset}

${colors.green}✓ Successful:${colors.reset} ${successCount}
${failCount > 0 ? `${colors.yellow}⚠ Failed:${colors.reset} ${failCount}` : ''}

${colors.blue}Next steps:${colors.reset}
  1. Review all changes
  2. Manually convert complex sx props
  3. Test each page
  4. Delete backup files (.tsx.bak) if everything works

${colors.blue}Useful commands:${colors.reset}
  # Find all backup files
  find src -name "*.tsx.bak"
  
  # Delete all backup files (after testing)
  find src -name "*.tsx.bak" -delete
  
  # Compare original and migrated
  git diff src/pages/unlock.tsx

See TAILWIND_MIGRATION_QUICK_GUIDE.md for manual conversion guide.
    `)
  }, 3000)
}

main()
