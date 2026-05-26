# 强制清理所有构建缓存
Write-Host "Cleaning build cache..." -ForegroundColor Yellow

# 1. 停止可能占用文件的进程
Write-Host "Stopping processes..." -ForegroundColor Cyan
Get-Process | Where-Object {$_.Path -like "*clashverge-clean*"} | Stop-Process -Force -ErrorAction SilentlyContinue

# 等待进程完全退出
Start-Sleep -Seconds 2

# 2. 删除 Rust 构建缓存
Write-Host "Removing Rust build cache..." -ForegroundColor Cyan
if (Test-Path "target") {
    Remove-Item "target" -Recurse -Force -ErrorAction SilentlyContinue
    if ($?) {
        Write-Host "✓ Removed target/" -ForegroundColor Green
    } else {
        Write-Host "✗ Failed to remove some files in target/" -ForegroundColor Red
        Write-Host "  Try closing all applications and run again" -ForegroundColor Yellow
    }
}

# 3. 删除 Node 构建缓存
Write-Host "Removing Node build cache..." -ForegroundColor Cyan
if (Test-Path "dist") {
    Remove-Item "dist" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "✓ Removed dist/" -ForegroundColor Green
}

if (Test-Path "node_modules\.vite") {
    Remove-Item "node_modules\.vite" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "✓ Removed node_modules/.vite/" -ForegroundColor Green
}

# 4. 清理 Cargo 缓存（可选，更彻底）
Write-Host "Cleaning Cargo cache..." -ForegroundColor Cyan
Push-Location src-tauri
cargo clean 2>$null
if ($?) {
    Write-Host "✓ Cargo clean completed" -ForegroundColor Green
} else {
    Write-Host "✗ Cargo clean failed (some files may be in use)" -ForegroundColor Red
}
Pop-Location

Write-Host ""
Write-Host "Cache cleaned! Now you can rebuild:" -ForegroundColor Green
Write-Host "  pnpm build" -ForegroundColor Cyan
Write-Host ""
Write-Host "This will ensure the new icon is used." -ForegroundColor Yellow
