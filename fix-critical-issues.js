import fs from 'fs';
import path from 'path';
import { glob } from 'glob';

console.log('修复关键问题...\n');

const files = await glob('src/**/*.{ts,tsx}', { absolute: true });
let fixedCount = 0;

for (const filePath of files) {
  let content = fs.readFileSync(filePath, 'utf8');
  const original = content;
  const fileName = path.basename(filePath);
  
  // 1. 修复 Menu 重复导入 (layout.tsx)
  if (fileName === 'layout.tsx' && filePath.includes('_layout')) {
    // 重命名 lucide-react 的 Menu 为 MenuIcon
    content = content.replace(
      /import \{ Menu \} from 'lucide-react'/,
      "import { Menu as MenuIcon } from 'lucide-react'"
    );
    // 替换使用
    content = content.replace(/<Menu /g, '<MenuIcon ');
    content = content.replace(/<\/Menu>/g, '</MenuIcon>');
  }
  
  // 2. 修复 Github 图标导入 (settings.tsx)
  if (fileName === 'settings.tsx') {
    content = content.replace(/\bGithub\b/g, 'Github');
    // 确保从 lucide-react 导入
    if (content.includes('Github') && !content.includes("import { Github }")) {
      const lucideImportRegex = /import\s+\{([^}]+)\}\s+from\s+['"]lucide-react['"]/;
      const match = content.match(lucideImportRegex);
      
      if (match) {
        const existingImports = match[1].split(',').map(i => i.trim());
        if (!existingImports.includes('Github')) {
          existingImports.push('Github');
          content = content.replace(match[0], `import { ${existingImports.join(', ')} } from 'lucide-react'`);
        }
      }
    }
  }
  
  // 3. 修复 Select options 属性 - 移除 children 时的 options 要求
  // 这个需要在 Select.tsx 中修复，这里跳过
  
  // 4. 修复 AlertCircle 类型使用 (use-dns-manager.ts)
  if (fileName === 'use-dns-manager.ts') {
    content = content.replace(/:\s*AlertCircle\b/g, ': typeof AlertCircle');
  }
  
  // 5. 修复 TextField onChange - 从 ChangeEvent 提取 value
  // 查找 onChange={(e) => setState(e)} 模式
  content = content.replace(
    /onChange=\{([^}]+)\}\s+\/\/.*setState/g,
    (match) => {
      if (match.includes('e.target.value')) {
        return match;
      }
      return match.replace(/\(e\)/, '(e)').replace(/setState\(e\)/, 'setState(e.target.value)');
    }
  );
  
  // 6. 移除不支持的属性
  // slotProps
  content = content.replace(/\s+slotProps=\{[^}]*\}/g, '');
  
  // component 属性
  content = content.replace(/\s+component=["'][^"']*["']/g, '');
  
  // titleAccess 属性
  content = content.replace(/\s+titleAccess=["'][^"']*["']/g, '');
  
  // 7. 修复 Grid 响应式属性 - 暂时注释掉
  // 这个需要更复杂的处理，暂时跳过
  
  if (content !== original) {
    fs.writeFileSync(filePath, content, 'utf8');
    fixedCount++;
    console.log(`✓ ${path.relative(process.cwd(), filePath)}`);
  }
}

console.log(`\n修复了 ${fixedCount} 个文件`);
console.log('\n⚠️  注意: Grid 响应式属性问题需要手动修复或重写 Grid 组件');
