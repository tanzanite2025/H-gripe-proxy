import fs from 'fs';
import path from 'path';
import { glob } from 'glob';

console.log('安全地移除 slotProps...\n');

const files = await glob('src/**/*.{ts,tsx}', { absolute: true });
let fixedCount = 0;

for (const filePath of files) {
  let content = fs.readFileSync(filePath, 'utf8');
  const original = content;
  
  // 只移除简单的 slotProps（不跨行的）
  // 匹配 slotProps={{ ... }} 但不跨多行
  content = content.replace(/\s+slotProps=\{\{[^}]*\}\}/g, '');
  
  // 对于 Dialog 的 slotProps.paper，转换为 className
  content = content.replace(
    /slotProps=\{\{\s*paper:\s*\{\s*className:\s*["']([^"']*)["']\s*\}\s*\}\}/g,
    'className="$1"'
  );
  
  if (content !== original) {
    fs.writeFileSync(filePath, content, 'utf8');
    fixedCount++;
    console.log(`✓ ${path.relative(process.cwd(), filePath)}`);
  }
}

console.log(`\n修复了 ${fixedCount} 个文件`);
