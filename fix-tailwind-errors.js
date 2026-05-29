#!/usr/bin/env node
/**
 * Tailwind Migration Error Fixer
 * Fixes TypeScript errors from MUI to Tailwind migration
 */

const fs = require('fs');
const path = require('path');
const glob = require('glob');

// MUI icon to Lucide icon mapping
const iconMapping = {
  'ExpandMore': 'ChevronDown',
  'ExpandLess': 'ChevronUp',
  'Close': 'X',
  'Delete': 'Trash2',
  'ContentCopy': 'Copy',
  'Visibility': 'Eye',
  'VisibilityOff': 'EyeOff',
  'Refresh': 'RefreshCw',
  'Add': 'Plus',
  'Remove': 'Minus',
  'Check': 'Check',
  'Settings': 'Settings',
  'Info': 'Info',
  'Warning': 'AlertTriangle',
  'Error': 'AlertCircle',
  'CheckCircle': 'CheckCircle',
  'Cancel': 'XCircle',
  'Download': 'Download',
  'Replay': 'RotateCcw',
  'Shuffle': 'Shuffle',
  'Repeat': 'Repeat',
};

function fixFile(filePath) {
  let content = fs.readFileSync(filePath, 'utf8');
  let modified = false;
  const originalContent = content;

  // Fix 1: Remove @emotion imports
  if (content.includes('@emotion/cache') || content.includes('@emotion/react')) {
    content = content.replace(/import\s+.*?from\s+['"]@emotion\/(cache|react)['"]\s*;?\n?/g, '');
    modified = true;
  }

  // Fix 2: Replace @mui/icons-material imports with lucide-react
  if (content.includes('@mui/icons-material')) {
    const muiIconImportRegex = /import\s+\{([^}]+)\}\s+from\s+['"]@mui\/icons-material['"]/g;
    const matches = [...content.matchAll(muiIconImportRegex)];
    
    if (matches.length > 0) {
      matches.forEach(match => {
        const icons = match[1].split(',').map(i => i.trim());
        const lucideIcons = icons.map(icon => iconMapping[icon] || icon).filter(Boolean);
        
        if (lucideIcons.length > 0) {
          const newImport = `import { ${lucideIcons.join(', ')} } from 'lucide-react'`;
          content = content.replace(match[0], newImport);
          modified = true;
        }
      });
    }
  }

  // Fix 3: Replace @mui/material imports
  if (content.includes('@mui/material')) {
    content = content.replace(/import\s+\{[^}]*\}\s+from\s+['"]@mui\/material['"]\s*;?\n?/g, '');
    modified = true;
  }

  // Fix 4: Fix Select onChange - remove .target.value
  content = content.replace(
    /onChange=\{[^}]*\(([^)]*)\)\s*=>\s*([^}]*?)\.target\.value/g,
    (match, param, prefix) => {
      return match.replace('.target.value', '');
    }
  );

  // Fix 5: Fix Select onChange with e.target
  content = content.replace(
    /onChange=\{\(e(?:vent)?\)\s*=>\s*([^}]+)e\.target\.value/g,
    'onChange={(value) => $1value'
  );

  // Fix 6: Remove labelId prop from Select
  content = content.replace(/\s+labelId=["'][^"']*["']/g, '');

  // Fix 7: Remove size prop from Select (not supported)
  content = content.replace(/\s+size=["'][^"']*["']/g, '');

  // Fix 8: Remove edge prop from Checkbox
  content = content.replace(/\s+edge=["'][^"']*["']/g, '');

  // Fix 9: Remove secondaryAction prop from ListItem
  content = content.replace(/\s+secondaryAction=\{[^}]*\}/g, '');

  // Fix 10: Remove onClick from ListItemText
  content = content.replace(/<ListItemText([^>]*)\s+onClick=\{[^}]*\}/g, '<ListItemText$1');

  // Fix 11: Remove secondaryClassName from ListItemText
  content = content.replace(/\s+secondaryClassName=["'][^"']*["']/g, '');

  // Fix 12: Remove style prop from ListItem
  content = content.replace(/<ListItem([^>]*)\s+style=\{[^}]*\}/g, '<ListItem$1');

  // Fix 13: Remove title prop from ListItemButton
  content = content.replace(/<ListItemButton([^>]*)\s+title=["'][^"']*["']/g, '<ListItemButton$1');

  // Fix 14: Remove message prop from Snackbar
  content = content.replace(/<Snackbar([^>]*)\s+message=\{[^}]*\}/g, '<Snackbar$1');

  // Fix 15: Remove style prop from Snackbar
  content = content.replace(/<Snackbar([^>]*)\s+style=\{[^}]*\}/g, '<Snackbar$1');

  // Fix 16: Remove disableEnforceFocus from Dialog
  content = content.replace(/\s+disableEnforceFocus=\{[^}]*\}/g, '');

  // Fix 17: Remove onClick from Paper
  content = content.replace(/<Paper([^>]*)\s+onClick=\{[^}]*\}/g, '<Paper$1');

  // Fix 18: Remove onClick from Chip
  content = content.replace(/<Chip([^>]*)\s+onClick=\{[^}]*\}/g, '<Chip$1');

  // Fix 19: Remove onDelete from Chip
  content = content.replace(/<Chip([^>]*)\s+onDelete=\{[^}]*\}/g, '<Chip$1');

  // Fix 20: Fix boolean | undefined to boolean
  content = content.replace(/checked=\{([^}]+)\}/g, (match, expr) => {
    if (!expr.includes('??') && !expr.includes('||')) {
      return `checked={${expr} ?? false}`;
    }
    return match;
  });

  // Fix 21: Add missing imports for Tailwind components
  const tailwindComponents = ['Switch', 'Select', 'TextField', 'Button', 'Chip', 'Dialog', 'Paper', 'ListItem', 'ListItemText', 'ListItemButton', 'Snackbar', 'Checkbox'];
  const usedComponents = [];
  
  tailwindComponents.forEach(comp => {
    const regex = new RegExp(`<${comp}[\\s>]`, 'g');
    if (regex.test(content)) {
      usedComponents.push(comp);
    }
  });

  if (usedComponents.length > 0) {
    // Check if there's already an import from tailwind
    const tailwindImportRegex = /import\s+\{([^}]+)\}\s+from\s+['"]@\/components\/tailwind['"]/;
    const match = content.match(tailwindImportRegex);
    
    if (match) {
      const existingImports = match[1].split(',').map(i => i.trim());
      const newImports = [...new Set([...existingImports, ...usedComponents])];
      content = content.replace(match[0], `import { ${newImports.join(', ')} } from '@/components/tailwind'`);
      modified = true;
    }
  }

  if (content !== originalContent) {
    fs.writeFileSync(filePath, content, 'utf8');
    return true;
  }

  return false;
}

// Main execution
const srcDir = path.join(__dirname, 'src');
const files = glob.sync('**/*.{ts,tsx}', { cwd: srcDir, absolute: true });

let fixedCount = 0;
let errorCount = 0;

console.log(`Found ${files.length} TypeScript files to process...\n`);

files.forEach(file => {
  try {
    if (fixFile(file)) {
      fixedCount++;
      console.log(`✓ Fixed: ${path.relative(process.cwd(), file)}`);
    }
  } catch (error) {
    errorCount++;
    console.error(`✗ Error processing ${path.relative(process.cwd(), file)}:`, error.message);
  }
});

console.log(`\n${'='.repeat(50)}`);
console.log(`Summary:`);
console.log(`  Total files: ${files.length}`);
console.log(`  Fixed: ${fixedCount}`);
console.log(`  Errors: ${errorCount}`);
console.log(`${'='.repeat(50)}\n`);
