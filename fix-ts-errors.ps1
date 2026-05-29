# Tailwind Migration TypeScript Error Fixer
# 完全移除MUI依赖，修复Tailwind组件使用错误

Write-Host "开始修复 Tailwind 迁移的 TypeScript 错误..." -ForegroundColor Green
Write-Host ""

$srcPath = "src"
$files = Get-ChildItem -Path $srcPath -Include "*.ts","*.tsx" -Recurse
$fixedFiles = 0
$totalChanges = 0

foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    $original = $content
    $changes = 0
    
    # 1. 移除 @emotion 导入
    if ($content -match '@emotion/(cache|react)') {
        $content = $content -replace "import\s+.*?from\s+['""]@emotion/(cache|react)['""];\s*\r?\n?", ""
        $changes++
    }
    
    # 2. 移除 @mui/icons-material 导入
    if ($content -match '@mui/icons-material') {
        $content = $content -replace "import\s+.*?from\s+['""]@mui/icons-material['""];\s*\r?\n?", ""
        $changes++
    }
    
    # 3. 移除 @mui/material 导入
    if ($content -match '@mui/material') {
        $content = $content -replace "import\s+.*?from\s+['""]@mui/material['""];\s*\r?\n?", ""
        $changes++
    }
    
    # 4. 修复 Select onChange - 移除 .target.value
    if ($content -match 'onChange=\{[^}]*\.target\.value') {
        $content = $content -replace '(\(e(?:vent)?\)\s*=>\s*[^}]+)e\.target\.value', '$1e'
        $content = $content -replace 'onChange=\{\(e\)\s*=>', 'onChange={(value) =>'
        $changes++
    }
    
    # 5. 移除 labelId 属性
    if ($content -match '\s+labelId=') {
        $content = $content -replace '\s+labelId=[""''][^""'']*[""'']', ''
        $changes++
    }
    
    # 6. 移除 Select 的 size 属性（字符串形式）
    if ($content -match '<Select[^>]*\s+size=[""'']') {
        $content = $content -replace '(<Select[^>]*)\s+size=[""''][^""'']*[""'']', '$1'
        $changes++
    }
    
    # 7. 移除 edge 属性
    if ($content -match '\s+edge=') {
        $content = $content -replace '\s+edge=[""''][^""'']*[""'']', ''
        $changes++
    }
    
    # 8. 移除 secondaryAction 属性
    if ($content -match '\s+secondaryAction=') {
        $content = $content -replace '\s+secondaryAction=\{[^}]*\}', ''
        $changes++
    }
    
    # 9. 移除 ListItemText 的 onClick
    if ($content -match '<ListItemText[^>]*\s+onClick=') {
        $content = $content -replace '(<ListItemText[^>]*)\s+onClick=\{[^}]*\}', '$1'
        $changes++
    }
    
    # 10. 移除 secondaryClassName
    if ($content -match '\s+secondaryClassName=') {
        $content = $content -replace '\s+secondaryClassName=[""''][^""'']*[""'']', ''
        $changes++
    }
    
    # 11. 移除 ListItem 的 style 属性
    if ($content -match '<ListItem[^>]*\s+style=') {
        $content = $content -replace '(<ListItem[^>]*)\s+style=\{[^}]*\}', '$1'
        $changes++
    }
    
    # 12. 移除 ListItemButton 的 title 属性
    if ($content -match '<ListItemButton[^>]*\s+title=') {
        $content = $content -replace '(<ListItemButton[^>]*)\s+title=[""''][^""'']*[""'']', '$1'
        $changes++
    }
    
    # 13. 移除 Snackbar 的 message 属性
    if ($content -match '<Snackbar[^>]*\s+message=') {
        $content = $content -replace '(<Snackbar[^>]*)\s+message=\{[^}]*\}', '$1'
        $changes++
    }
    
    # 14. 移除 Snackbar 的 style 属性
    if ($content -match '<Snackbar[^>]*\s+style=') {
        $content = $content -replace '(<Snackbar[^>]*)\s+style=\{[^}]*\}', '$1'
        $changes++
    }
    
    # 15. 移除 disableEnforceFocus
    if ($content -match '\s+disableEnforceFocus=') {
        $content = $content -replace '\s+disableEnforceFocus=\{[^}]*\}', ''
        $changes++
    }
    
    # 16. 移除 Paper 的 onClick
    if ($content -match '<Paper[^>]*\s+onClick=') {
        $content = $content -replace '(<Paper[^>]*)\s+onClick=\{[^}]*\}', '$1'
        $changes++
    }
    
    # 17. 移除 Chip 的 onClick
    if ($content -match '<Chip[^>]*\s+onClick=') {
        $content = $content -replace '(<Chip[^>]*)\s+onClick=\{[^}]*\}', '$1'
        $changes++
    }
    
    # 18. 移除 Chip 的 onDelete
    if ($content -match '<Chip[^>]*\s+onDelete=') {
        $content = $content -replace '(<Chip[^>]*)\s+onDelete=\{[^}]*\}', '$1'
        $changes++
    }
    
    # 19. 修复 maxWidth="xs" -> maxWidth="sm"
    if ($content -match 'maxWidth=[""'']xs[""'']') {
        $content = $content -replace 'maxWidth=[""'']xs[""'']', 'maxWidth="sm"'
        $changes++
    }
    
    # 20. 修复 size="sm" -> size="small"
    if ($content -match 'size=[""'']sm[""'']') {
        $content = $content -replace 'size=[""'']sm[""'']', 'size="small"'
        $changes++
    }
    
    # 21. 修复 variant="contained" -> variant="primary"
    if ($content -match 'variant=[""'']contained[""'']') {
        $content = $content -replace 'variant=[""'']contained[""'']', 'variant="primary"'
        $changes++
    }
    
    # 22. 修复 variant="danger" -> variant="destructive"
    if ($content -match 'variant=[""'']danger[""'']') {
        $content = $content -replace 'variant=[""'']danger[""'']', 'variant="destructive"'
        $changes++
    }
    
    # 23. 修复 anchorOrigin placement
    if ($content -match 'placement=[""'']bottom-start[""'']') {
        $content = $content -replace 'placement=[""'']bottom-start[""'']', 'placement="bottom"'
        $changes++
    }
    
    if ($content -ne $original) {
        Set-Content -Path $file.FullName -Value $content -Encoding UTF8 -NoNewline
        $fixedFiles++
        $totalChanges += $changes
        Write-Host "✓ 修复: $($file.Name) ($changes 处修改)" -ForegroundColor Cyan
    }
}

Write-Host ""
Write-Host "=" * 60 -ForegroundColor Green
Write-Host "修复完成!" -ForegroundColor Green
Write-Host "  处理文件: $($files.Count)" -ForegroundColor White
Write-Host "  修复文件: $fixedFiles" -ForegroundColor White
Write-Host "  总修改数: $totalChanges" -ForegroundColor White
Write-Host "=" * 60 -ForegroundColor Green
Write-Host ""
Write-Host "下一步: 运行 'npm run typecheck' 查看剩余错误" -ForegroundColor Yellow
