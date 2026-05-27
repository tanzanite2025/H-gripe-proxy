#!/usr/bin/env node

/**
 * Emotion 样式注入验证脚本
 * 用于检查 production 构建后的样式完整性
 */

import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = fileURLToPath(new URL('.', import.meta.url))
const distPath = join(__dirname, '..', 'dist')
const indexHtmlPath = join(distPath, 'index.html')

console.log('🔍 开始验证 Emotion 样式注入...\n')

// 检查 dist 目录是否存在
if (!existsSync(distPath)) {
  console.error('❌ dist 目录不存在，请先运行 pnpm build')
  process.exit(1)
}

// 检查 index.html 是否存在
if (!existsSync(indexHtmlPath)) {
  console.error('❌ dist/index.html 不存在')
  process.exit(1)
}

const indexHtml = readFileSync(indexHtmlPath, 'utf-8')

// 验证项
const checks = [
  {
    name: 'Emotion 插入点',
    pattern: /<meta\s+name="emotion-insertion-point"/,
    required: true,
    description: '确保 Emotion 样式注入点存在',
  },
  {
    name: 'Emotion 样式标签',
    pattern: /<style\s+data-emotion="mui/,
    required: true,
    description: '确保 MUI/Emotion 样式已注入',
  },
  {
    name: 'MuiSvgIcon 样式',
    pattern: /MuiSvgIcon/,
    required: true,
    description: '确保 MUI 图标样式存在',
  },
  {
    name: 'MuiButton 样式',
    pattern: /MuiButton/,
    required: true,
    description: '确保 MUI 按钮样式存在',
  },
  {
    name: 'CSS 变量',
    pattern: /--primary-main/,
    required: true,
    description: '确保主题 CSS 变量存在',
  },
]

let allPassed = true

for (const check of checks) {
  const passed = check.pattern.test(indexHtml)
  const icon = passed ? '✅' : check.required ? '❌' : '⚠️'
  console.log(`${icon} ${check.name}: ${passed ? '通过' : '失败'}`)
  console.log(`   ${check.description}`)
  
  if (!passed && check.required) {
    allPassed = false
  }
}

console.log('\n' + '='.repeat(50))

if (allPassed) {
  console.log('✅ 所有验证通过！Emotion 样式注入正常。')
  process.exit(0)
} else {
  console.log('❌ 验证失败！请检查以下问题：')
  console.log('   1. 确认已安装 @emotion/babel-plugin')
  console.log('   2. 检查 vite.config.mts 中的 React 插件配置')
  console.log('   3. 检查 base-emotion-style-chain.tsx 中的 speedy 配置')
  console.log('\n详细修复方案请参考：EMOTION_STYLE_INJECTION_FIX.md')
  process.exit(1)
}
