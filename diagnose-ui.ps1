# UI 问题快速诊断脚本
param(
    [switch]$Detailed
)

Write-Host "=== Clash Verge UI 诊断工具 ===" -ForegroundColor Cyan
Write-Host ""

$issues = @()

# 1. 检查构建输出
Write-Host "[1/5] 检查构建输出..." -ForegroundColor Yellow
if (!(Test-Path "dist")) {
    Write-Host "  ✗ dist 目录不存在！请运行: pnpm run web:build" -ForegroundColor Red
    $issues += "dist 目录缺失"
} else {
    $cssFiles = Get-ChildItem "dist\assets\*.css" -ErrorAction SilentlyContinue
    if (!$cssFiles) {
        Write-Host "  ✗ 未找到 CSS 文件！" -ForegroundColor Red
        $issues += "CSS 文件缺失"
    } else {
        Write-Host "  ✓ 找到 $($cssFiles.Count) 个 CSS 文件" -ForegroundColor Green
        if ($Detailed) {
            $cssFiles | ForEach-Object {
                Write-Host "    - $($_.Name) ($([math]::Round($_.Length/1KB,2)) KB)" -ForegroundColor Gray
            }
        }
    }
}

# 2. 检查 CSS 内容
Write-Host "[2/5] 检查 CSS 内容..." -ForegroundColor Yellow
try {
    $css = Get-Content "dist\assets\index-*.css" -Raw -ErrorAction Stop
    
    $checks = @{
        "UDS 样式 (.uds-*)" = $css -match '\.uds-'
        "CSS 变量 (--primary-main)" = $css -match '--primary-main'
        "字体定义 (font-family)" = $css -match 'font-family'
        "布局样式 (.layout)" = $css -match '\.layout'
        "Google Fonts 导入" = $css -match '@import.*fonts\.googleapis'
    }
    
    $allPassed = $true
    foreach ($check in $checks.GetEnumerator()) {
        if ($check.Value) {
            Write-Host "  ✓ $($check.Key)" -ForegroundColor Green
        } else {
            Write-Host "  ✗ $($check.Key)" -ForegroundColor Red
            $issues += "CSS 缺少: $($check.Key)"
            $allPassed = $false
        }
    }
    
    if ($allPassed) {
        Write-Host "  ✓ CSS 内容完整" -ForegroundColor Green
    }
} catch {
    Write-Host "  ✗ 无法读取 CSS 文件" -ForegroundColor Red
    $issues += "CSS 文件读取失败"
}

# 3. 检查 HTML
Write-Host "[3/5] 检查 HTML..." -ForegroundColor Yellow
if (Test-Path "dist\index.html") {
    $html = Get-Content "dist\index.html" -Raw
    
    $checks = @{
        "CSS 链接" = $html -match '<link.*stylesheet.*\.css'
        "JS 脚本" = $html -match '<script.*\.js'
        "Emotion meta 标签" = $html -match 'emotion-insertion-point'
    }
    
    foreach ($check in $checks.GetEnumerator()) {
        if ($check.Value) {
            Write-Host "  ✓ $($check.Key)" -ForegroundColor Green
        } else {
            Write-Host "  ✗ $($check.Key)" -ForegroundColor Red
            $issues += "HTML 缺少: $($check.Key)"
        }
    }
    
    if ($Detailed) {
        Write-Host "`n  HTML 资源引用:" -ForegroundColor Gray
        $html | Select-String -Pattern '<link.*href="([^"]+\.css)"' -AllMatches | ForEach-Object {
            $_.Matches | ForEach-Object {
                Write-Host "    CSS: $($_.Groups[1].Value)" -ForegroundColor Gray
            }
        }
        $html | Select-String -Pattern '<script.*src="([^"]+\.js)"' -AllMatches | ForEach-Object {
            $_.Matches | ForEach-Object {
                Write-Host "    JS:  $($_.Groups[1].Value)" -ForegroundColor Gray
            }
        }
    }
} else {
    Write-Host "  ✗ index.html 不存在" -ForegroundColor Red
    $issues += "index.html 缺失"
}

# 4. 检查 CSP 配置
Write-Host "[4/5] 检查 CSP 配置..." -ForegroundColor Yellow
try {
    $tauriConf = Get-Content "src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
    $csp = $tauriConf.app.security.csp
    
    if (!$csp) {
        Write-Host "  ✗ CSP 未配置" -ForegroundColor Red
        $issues += "CSP 未配置"
    } else {
        $checks = @{
            "允许 Google Fonts CSS" = $csp -match 'fonts\.googleapis\.com'
            "允许 Google Fonts 字体" = $csp -match 'fonts\.gstatic\.com'
            "允许内联样式" = $csp -match "style-src.*'unsafe-inline'"
            "允许内联脚本" = $csp -match "script-src.*'unsafe-inline'"
        }
        
        foreach ($check in $checks.GetEnumerator()) {
            if ($check.Value) {
                Write-Host "  ✓ $($check.Key)" -ForegroundColor Green
            } else {
                Write-Host "  ✗ $($check.Key)" -ForegroundColor Red
                $issues += "CSP 配置问题: $($check.Key)"
            }
        }
        
        if ($Detailed) {
            Write-Host "`n  完整 CSP:" -ForegroundColor Gray
            Write-Host "    $csp" -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "  ✗ 无法读取 tauri.conf.json" -ForegroundColor Red
    $issues += "tauri.conf.json 读取失败"
}

# 5. 检查样式文件源
Write-Host "[5/5] 检查样式源文件..." -ForegroundColor Yellow
$styleFiles = @(
    "src\assets\styles\index.scss",
    "src\assets\styles\layout.scss",
    "src\assets\styles\page.scss",
    "src\assets\styles\font.scss"
)

foreach ($file in $styleFiles) {
    if (Test-Path $file) {
        Write-Host "  ✓ $file" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $file 不存在" -ForegroundColor Red
        $issues += "样式源文件缺失: $file"
    }
}

# 总结
Write-Host ""
Write-Host "=== 诊断结果 ===" -ForegroundColor Cyan

if ($issues.Count -eq 0) {
    Write-Host "✓ 未发现问题！" -ForegroundColor Green
    Write-Host ""
    Write-Host "如果 UI 仍然错位，请尝试:" -ForegroundColor Yellow
    Write-Host "  1. 清理 WebView2 缓存:" -ForegroundColor White
    Write-Host "     Remove-Item `"`$env:LOCALAPPDATA\Clash Verge Optimized\EBWebView`" -Recurse -Force" -ForegroundColor Gray
    Write-Host "  2. 重新构建:" -ForegroundColor White
    Write-Host "     .\clean-build.ps1" -ForegroundColor Gray
    Write-Host "     pnpm build" -ForegroundColor Gray
    Write-Host "  3. 在应用中按 F12 查看浏览器控制台错误" -ForegroundColor White
} else {
    Write-Host "✗ 发现 $($issues.Count) 个问题:" -ForegroundColor Red
    Write-Host ""
    $issues | ForEach-Object {
        Write-Host "  • $_" -ForegroundColor Red
    }
    Write-Host ""
    Write-Host "建议操作:" -ForegroundColor Yellow
    Write-Host "  1. 运行: .\clean-build.ps1" -ForegroundColor White
    Write-Host "  2. 运行: pnpm run web:build" -ForegroundColor White
    Write-Host "  3. 重新运行此诊断: .\diagnose-ui.ps1" -ForegroundColor White
}

Write-Host ""
Write-Host "详细诊断: .\diagnose-ui.ps1 -Detailed" -ForegroundColor Gray
Write-Host "完整文档: UI_DEBUG_CHECKLIST.md" -ForegroundColor Gray
