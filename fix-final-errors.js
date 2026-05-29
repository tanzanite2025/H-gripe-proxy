import fs from 'fs';
import path from 'path';
import { glob } from 'glob';

console.log('修复最终的TypeScript错误...\n');

const files = await glob('src/**/*.{ts,tsx}', { absolute: true });
let fixedCount = 0;

for (const filePath of files) {
  let content = fs.readFileSync(filePath, 'utf8');
  const original = content;
  const fileName = path.basename(filePath);
  
  // 1. 修复 TextField onChange - 确保事件类型正确
  // 对于 type="number" 的 TextField，onChange 应该接收 ChangeEvent
  if (content.includes('TextField') && content.includes('type="number"')) {
    // 这些已经是正确的，不需要修改
  }
  
  // 2. 修复 base-dialog.tsx 的 onClose 可选问题
  if (fileName === 'base-dialog.tsx') {
    // 为 onClose 添加默认值或修复类型
    content = content.replace(
      /onClose=\{onClose\}/g,
      'onClose={onClose || (() => {})}'
    );
  }
  
  // 3. 修复 IconButton 的 aria-label 类型
  content = content.replace(
    /aria-label=\{([^}]+)\}/g,
    (match, expr) => {
      // 如果表达式可能是 undefined，添加默认值
      if (expr.includes('?') || expr.includes('undefined')) {
        return `aria-label={${expr} || ''}`;
      }
      return match;
    }
  );
  
  // 4. 修复 Tooltip title 类型 - 确保不是 ReactNode
  content = content.replace(
    /<Tooltip\s+title=\{([^}]+)\}/g,
    (match, expr) => {
      // 如果是复杂表达式，确保转换为字符串
      if (expr.includes('||') && !expr.includes('String(')) {
        return `<Tooltip title={String(${expr})}`;
      }
      return match;
    }
  );
  
  // 5. 修复 placement 属性值
  content = content.replace(/placement="bottom-start"/g, 'placement="bottom"');
  content = content.replace(/placement="top-start"/g, 'placement="top"');
  content = content.replace(/placement="left-start"/g, 'placement="left"');
  content = content.replace(/placement="right-start"/g, 'placement="right"');
  
  // 6. 添加缺失的 React 导入（如果使用了 ChangeEvent 但没有导入）
  if (content.includes('ChangeEvent<') && !content.includes("import") && !content.includes("from 'react'")) {
    const reactImport = "import { ChangeEvent } from 'react'\n";
    content = reactImport + content;
  }
  
  // 7. 修复 ViewModuleRounded 和 CodeRounded 图标（base-split-chip-editor）
  if (fileName === 'base-split-chip-editor.tsx') {
    // 替换 MUI 图标为 lucide-react
    content = content.replace(/ViewModuleRounded/g, 'LayoutGrid');
    content = content.replace(/CodeRounded/g, 'Code');
    
    // 确保有导入
    if (!content.includes('LayoutGrid') || !content.includes('Code')) {
      const lucideImportRegex = /import\s+\{([^}]+)\}\s+from\s+['"]lucide-react['"]/;
      const match = content.match(lucideImportRegex);
      
      if (match) {
        const existingImports = match[1].split(',').map(i => i.trim());
        const newImports = [...new Set([...existingImports, 'LayoutGrid', 'Code'])];
        content = content.replace(match[0], `import { ${newImports.join(', ')} } from 'lucide-react'`);
      } else {
        const importStatement = "import { LayoutGrid, Code } from 'lucide-react'\n";
        const firstImportMatch = content.match(/^import\s+/m);
        if (firstImportMatch) {
          const insertPos = content.indexOf('\n', firstImportMatch.index) + 1;
          content = content.slice(0, insertPos) + importStatement + content.slice(insertPos);
        }
      }
    }
  }
  
  if (content !== original) {
    fs.writeFileSync(filePath, content, 'utf8');
    fixedCount++;
    console.log(`✓ ${path.relative(process.cwd(), filePath)}`);
  }
}

console.log(`\n修复了 ${fixedCount} 个文件`);
