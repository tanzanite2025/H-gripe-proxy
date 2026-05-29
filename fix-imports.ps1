# 第一步：只修复导入问题
# 移除MUI相关的导入，不修改组件使用

Write-Host "修复 MUI 导入..." -ForegroundColor Green

$files = Get-ChildItem -Path "src" -Include "*.ts","*.tsx" -Recurse
$fixed = 0

foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    $original = $content
    
    # 只移除导入语句
    $content = $content -replace "import\s+[^;]+from\s+['""]@emotion/(cache|react)['""];?\s*\r?\n", ""
    $content = $content -replace "import\s+[^;]+from\s+['""]@mui/icons-material['""];?\s*\r?\n", ""
    $content = $content -replace "import\s+[^;]+from\s+['""]@mui/material['""];?\s*\r?\n", ""
    
    if ($content -ne $original) {
        Set-Content -Path $file.FullName -Value $content -Encoding UTF8 -NoNewline
        $fixed++
        Write-Host "✓ $($file.Name)" -ForegroundColor Cyan
    }
}

Write-Host "`n修复了 $fixed 个文件的导入" -ForegroundColor Green
