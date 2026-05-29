import fs from 'fs';
import path from 'path';
import { glob } from 'glob';

console.log('修复剩余的TypeScript问题...\n');

const files = await glob('src/**/*.{ts,tsx}', { absolute: true });
let fixedCount = 0;

for (const filePath of files) {
  let content = fs.readFileSync(filePath, 'utf8');
  const original = content;
  const fileName = path.basename(filePath);
  
  // 1. 修复 SelectChangeEvent 类型
  if (content.includes('SelectChangeEvent')) {
    // 移除 SelectChangeEvent 导入
    content = content.replace(/import\s+\{[^}]*SelectChangeEvent[^}]*\}\s+from\s+['"]@\/components\/tailwind\/Select['"]\s*;?\n?/g, '');
    
    // 替换 SelectChangeEvent<string> 为简单的函数签名
    content = content.replace(/\(event:\s*SelectChangeEvent<[^>]+>\)/g, '(value: string | number)');
    content = content.replace(/:\s*SelectChangeEvent<[^>]+>/g, ': (value: string | number) => void');
    
    console.log(`✓ ${fileName} - 修复 SelectChangeEvent`);
  }
  
  // 2. 添加 DnD Kit 导入 (connection-column-manager.tsx)
  if (fileName === 'connection-column-manager.tsx' && content.includes('useSensors') && !content.includes('@dnd-kit/core')) {
    const dndImports = `import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import {
  arrayMove,
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
`;
    
    // 在第一个import之后添加
    const firstImportMatch = content.match(/^import\s+/m);
    if (firstImportMatch) {
      const insertPos = content.indexOf('\n', firstImportMatch.index) + 1;
      content = content.slice(0, insertPos) + dndImports + content.slice(insertPos);
      console.log(`✓ ${fileName} - 添加 DnD Kit 导入`);
    }
  }
  
  // 3. 添加 React hooks 导入 (use-graph-renderer.ts)
  if (fileName === 'use-graph-renderer.ts' && content.includes('useRef') && !content.includes("from 'react'")) {
    const reactImport = "import { useRef, useCallback, useEffect } from 'react'\n";
    
    // 在文件开头添加
    content = reactImport + content;
    console.log(`✓ ${fileName} - 添加 React hooks 导入`);
  }
  
  // 4. 添加 useTheme 导入
  if (content.includes('useTheme()') && !content.includes("import { useTheme }")) {
    // 检查是否有其他从 @/services/theme 的导入
    const themeImportRegex = /import\s+\{([^}]+)\}\s+from\s+['"]@\/services\/theme['"]/;
    const match = content.match(themeImportRegex);
    
    if (match) {
      const existingImports = match[1].split(',').map(i => i.trim());
      if (!existingImports.includes('useTheme')) {
        existingImports.push('useTheme');
        content = content.replace(match[0], `import { ${existingImports.join(', ')} } from '@/services/theme'`);
        console.log(`✓ ${fileName} - 添加 useTheme 导入`);
      }
    } else {
      // 添加新的导入
      const themeImport = "import { useTheme } from '@/services/theme'\n";
      const firstImportMatch = content.match(/^import\s+/m);
      if (firstImportMatch) {
        const insertPos = content.indexOf('\n', firstImportMatch.index) + 1;
        content = content.slice(0, insertPos) + themeImport + content.slice(insertPos);
        console.log(`✓ ${fileName} - 添加 useTheme 导入`);
      }
    }
  }
  
  // 5. 修复 TextField onChange 事件
  if (content.includes('TextField') && content.includes('onChange={(e)')) {
    // 这个已经在之前的脚本中处理了，跳过
  }
  
  // 6. 修复 Chip onDelete 属性 - 移除它
  content = content.replace(/<Chip([^>]*)\s+onDelete=\{[^}]*\}/g, '<Chip$1');
  
  // 7. 修复 boolean | undefined 类型
  // 为 checked 属性添加默认值
  content = content.replace(/checked=\{([^}]+)\}/g, (match, expr) => {
    // 如果已经有默认值处理，跳过
    if (expr.includes('??') || expr.includes('||') || expr.includes('!')) {
      return match;
    }
    // 如果是简单的布尔值，添加默认值
    if (expr.match(/^[a-zA-Z_$][a-zA-Z0-9_$.]*$/)) {
      return `checked={${expr} ?? false}`;
    }
    return match;
  });
  
  if (content !== original) {
    fs.writeFileSync(filePath, content, 'utf8');
    fixedCount++;
  }
}

console.log(`\n修复了 ${fixedCount} 个文件`);
