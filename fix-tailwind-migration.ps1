# Tailwind Migration Fix Script
# This script fixes common TypeScript errors from MUI to Tailwind migration

Write-Host "Starting Tailwind migration fixes..." -ForegroundColor Green

# Get all TypeScript/TSX files in src directory
$files = Get-ChildItem -Path "src" -Include "*.ts","*.tsx" -Recurse

$totalFiles = 0
$totalChanges = 0

foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    $originalContent = $content
    $fileChanges = 0
    
    # Fix 1: Switch onCheckedChange - already supported, just need to add type
    # No change needed, component already supports it
    
    # Fix 2: Remove fullWidth from Select (it's already supported)
    # No change needed
    
    # Fix 3: Remove fullWidth from TextField (it's already supported)
    # No change needed
    
    # Fix 4: Fix Button variant "contained" -> already handled in component
    # No change needed
    
    # Fix 5: Fix Button variant "danger" -> already handled in component
    # No change needed
    
    # Fix 6: Fix size "sm" -> already handled in Button component
    # No change needed
    
    # Fix 7: Fix Select onChange to handle the event properly
    $content = $content -replace 'onChange=\{([^}]+)\.target\.value\}', 'onChange={$1}'
    
    # Fix 8: Remove @emotion imports (not needed)
    if ($content -match '@emotion/(cache|react)') {
        Write-Host "  Found @emotion import in $($file.Name)" -ForegroundColor Yellow
    }
    
    # Fix 9: Fix labelId -> label (Select component)
    # This needs manual review as labelId is not supported
    
    # Fix 10: Fix edge prop on Checkbox (not supported in Tailwind version)
    # This needs manual review
    
    if ($content -ne $originalContent) {
        $fileChanges = ($originalContent.Length - $content.Length)
        Set-Content -Path $file.FullName -Value $content -Encoding UTF8 -NoNewline
        $totalFiles++
        $totalChanges++
        Write-Host "  Fixed: $($file.Name)" -ForegroundColor Cyan
    }
}

Write-Host "`nSummary:" -ForegroundColor Green
Write-Host "  Files processed: $($files.Count)" -ForegroundColor White
Write-Host "  Files modified: $totalFiles" -ForegroundColor White
Write-Host "  Total changes: $totalChanges" -ForegroundColor White

Write-Host "`nNote: Some errors require manual fixes:" -ForegroundColor Yellow
Write-Host "  1. @emotion imports need to be removed or replaced" -ForegroundColor Yellow
Write-Host "  2. @mui/icons-material imports need to be replaced with lucide-react" -ForegroundColor Yellow
Write-Host "  3. Some component props may need manual adjustment" -ForegroundColor Yellow
