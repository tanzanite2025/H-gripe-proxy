import fs from 'fs';
import path from 'path';
import { glob } from 'glob';

console.log('修复 Tailwind 组件属性...\n');

const files = await glob('src/**/*.{ts,tsx}', { absolute: true });
let fixedCount = 0;

files.forEach(filePath => {
  let content = fs.readFileSync(filePath, 'utf8');
  const original = content;
  
  // 1. 修复 Select onChange - 只修复明确的 e.target.value 模式
  content = content.replace(
    /onChange=\{\(e\)\s*=>\s*([^}]+)e\.target\.value/g,
    'onChange={(value) => $1value'
  );
  
  // 2. 修复 variant="contained" -> variant="primary"
  content = content.replace(
    /variant="contained"/g,
    'variant="primary"'
  );
  
  // 3. 修复 variant="danger" -> variant="destructive"
  content = content.replace(
    /variant="danger"/g,
    'variant="destructive"'
  );
  
  // 4. 修复 size="sm" -> size="small"
  content = content.replace(
    /size="sm"/g,
    'size="small"'
  );
  
  // 5. 修复 maxWidth="xs" -> maxWidth="sm"
  content = content.replace(
    /maxWidth="xs"/g,
    'maxWidth="sm"'
  );
  
  // 6. 修复 placement="bottom-start" -> placement="bottom"
  content = content.replace(
    /placement="bottom-start"/g,
    'placement="bottom"'
  );
  
  if (content !== original) {
    fs.writeFileSync(filePath, content, 'utf8');
    fixedCount++;
    console.log(`✓ ${path.relative(process.cwd(), filePath)}`);
  }
});

console.log(`\n修复了 ${fixedCount} 个文件`);
