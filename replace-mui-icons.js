import fs from 'fs';
import path from 'path';
import { glob } from 'glob';

console.log('替换 MUI 图标为 Lucide React 图标...\n');

// MUI 图标到 Lucide 图标的映射
const iconMap = {
  // 基础图标
  'CloseRounded': 'X',
  'Close': 'X',
  'Add': 'Plus',
  'Remove': 'Minus',
  'Delete': 'Trash2',
  'DeleteOutlined': 'Trash2',
  'Edit': 'Edit',
  'EditOutlined': 'Edit2',
  'Save': 'Save',
  'SaveOutlined': 'Save',
  'Copy': 'Copy',
  'ContentCopy': 'Copy',
  
  // 箭头和方向
  'ArrowUpwardRounded': 'ArrowUp',
  'ArrowDownwardRounded': 'ArrowDown',
  'ArrowForward': 'ArrowRight',
  'ArrowBack': 'ArrowLeft',
  'ExpandMore': 'ChevronDown',
  'ExpandLess': 'ChevronUp',
  'ChevronRight': 'ChevronRight',
  'ChevronLeft': 'ChevronLeft',
  
  // 状态图标
  'Check': 'Check',
  'CheckCircle': 'CheckCircle',
  'CheckCircleOutlined': 'CheckCircle',
  'Error': 'AlertCircle',
  'ErrorOutlined': 'AlertCircle',
  'Warning': 'AlertTriangle',
  'WarningOutlined': 'AlertTriangle',
  'Info': 'Info',
  'InfoOutlined': 'Info',
  
  // 可见性
  'Visibility': 'Eye',
  'VisibilityOutlined': 'Eye',
  'VisibilityOff': 'EyeOff',
  'VisibilityOffOutlined': 'EyeOff',
  
  // 刷新和重复
  'Refresh': 'RefreshCw',
  'RefreshOutlined': 'RefreshCw',
  'Replay': 'RotateCcw',
  'Repeat': 'Repeat',
  'Shuffle': 'Shuffle',
  
  // 网络和连接
  'Link': 'Link',
  'LinkRounded': 'Link',
  'CloudUpload': 'CloudUpload',
  'CloudUploadRounded': 'CloudUpload',
  'CloudDownload': 'CloudDownload',
  'CloudDownloadRounded': 'CloudDownload',
  'Download': 'Download',
  'Upload': 'Upload',
  
  // 位置和地图
  'LocationOn': 'MapPin',
  'LocationOnOutlined': 'MapPin',
  'Place': 'MapPin',
  
  // 安全
  'Security': 'Shield',
  'SecurityOutlined': 'Shield',
  'Lock': 'Lock',
  'LockOutlined': 'Lock',
  
  // 设备和硬件
  'Computer': 'Monitor',
  'ComputerRounded': 'Monitor',
  'Memory': 'Cpu',
  'MemoryRounded': 'Cpu',
  'Storage': 'HardDrive',
  
  // 速度和性能
  'Speed': 'Gauge',
  'SpeedOutlined': 'Gauge',
  'Timer': 'Timer',
  
  // 设置和工具
  'Settings': 'Settings',
  'SettingsOutlined': 'Settings',
  'Build': 'Wrench',
  'Troubleshoot': 'Wrench',
  'TroubleshootRounded': 'Wrench',
  
  // 导航
  'Home': 'Home',
  'Menu': 'Menu',
  'MoreVert': 'MoreVertical',
  'MoreHoriz': 'MoreHorizontal',
  
  // 语言和全球
  'Language': 'Globe',
  'LanguageRounded': 'Globe',
  'Public': 'Globe',
  
  // 方向和路由
  'Directions': 'Navigation',
  'DirectionsRounded': 'Navigation',
  'MultipleStop': 'GitBranch',
  'MultipleStopRounded': 'GitBranch',
  
  // 帮助
  'Help': 'HelpCircle',
  'HelpOutline': 'HelpCircle',
  'HelpOutlineRounded': 'HelpCircle',
  
  // 拖拽
  'DragIndicator': 'GripVertical',
  'DragIndicatorRounded': 'GripVertical',
  
  // 信号
  'Signal': 'Signal',
  'SignalNone': 'SignalZero',
  'SignalWeak': 'SignalLow',
  'SignalMedium': 'SignalMedium',
  'SignalGood': 'SignalHigh',
  'SignalStrong': 'Signal',
  'SignalError': 'WifiOff',
};

const files = await glob('src/**/*.{ts,tsx}', { absolute: true });
let fixedCount = 0;
let iconReplacements = 0;

for (const filePath of files) {
  let content = fs.readFileSync(filePath, 'utf8');
  const original = content;
  let fileIconCount = 0;
  
  // 替换图标使用
  for (const [muiIcon, lucideIcon] of Object.entries(iconMap)) {
    const regex = new RegExp(`<${muiIcon}([\\s/>])`, 'g');
    if (regex.test(content)) {
      content = content.replace(regex, `<${lucideIcon}$1`);
      fileIconCount++;
    }
  }
  
  // 如果文件有图标替换，确保有lucide-react导入
  if (fileIconCount > 0) {
    const usedIcons = new Set();
    
    // 找出使用了哪些Lucide图标
    for (const lucideIcon of Object.values(iconMap)) {
      const regex = new RegExp(`<${lucideIcon}[\\s/>]`, 'g');
      if (regex.test(content)) {
        usedIcons.add(lucideIcon);
      }
    }
    
    if (usedIcons.size > 0) {
      // 检查是否已有lucide-react导入
      const lucideImportRegex = /import\s+\{([^}]+)\}\s+from\s+['"]lucide-react['"]/;
      const match = content.match(lucideImportRegex);
      
      if (match) {
        // 合并现有导入
        const existingImports = match[1].split(',').map(i => i.trim());
        const allImports = [...new Set([...existingImports, ...usedIcons])].sort();
        content = content.replace(match[0], `import { ${allImports.join(', ')} } from 'lucide-react'`);
      } else {
        // 添加新导入
        const imports = [...usedIcons].sort().join(', ');
        const importStatement = `import { ${imports} } from 'lucide-react'\n`;
        
        // 在第一个import之后添加
        const firstImportMatch = content.match(/^import\s+/m);
        if (firstImportMatch) {
          const insertPos = content.indexOf('\n', firstImportMatch.index) + 1;
          content = content.slice(0, insertPos) + importStatement + content.slice(insertPos);
        } else {
          content = importStatement + content;
        }
      }
    }
  }
  
  if (content !== original) {
    fs.writeFileSync(filePath, content, 'utf8');
    fixedCount++;
    iconReplacements += fileIconCount;
    console.log(`✓ ${path.relative(process.cwd(), filePath)} (${fileIconCount} 个图标)`);
  }
}

console.log(`\n修复了 ${fixedCount} 个文件，替换了 ${iconReplacements} 个图标`);
