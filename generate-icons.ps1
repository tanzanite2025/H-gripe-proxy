# Icon Generation Script
# Generate all required icon files from icon.ico

Add-Type -AssemblyName System.Drawing

$iconPath = "icon.ico"
$outputDir = "."

Write-Host "Starting icon generation from $iconPath..." -ForegroundColor Cyan

if (-not (Test-Path $iconPath)) {
    Write-Host "Error: Cannot find $iconPath" -ForegroundColor Red
    exit 1
}

try {
    $icon = [System.Drawing.Icon]::new($iconPath)
    Write-Host "Successfully loaded ICO file" -ForegroundColor Green
} catch {
    Write-Host "Error: Cannot load ICO file - $_" -ForegroundColor Red
    exit 1
}

function Get-LargestIconImage {
    param($icon)
    
    $largestBitmap = $null
    $sizes = @(256, 128, 64, 48, 32, 16)
    
    foreach ($size in $sizes) {
        try {
            $bitmap = [System.Drawing.Bitmap]::new($size, $size)
            $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.DrawIcon($icon, 0, 0, $size, $size)
            $graphics.Dispose()
            $largestBitmap = $bitmap
            Write-Host "  Found ${size}x${size} size" -ForegroundColor Gray
            break
        } catch {
            if ($bitmap) { $bitmap.Dispose() }
            continue
        }
    }
    
    if (-not $largestBitmap) {
        $largestBitmap = $icon.ToBitmap()
    }
    
    return $largestBitmap
}

function Resize-Image {
    param(
        [System.Drawing.Bitmap]$sourceBitmap,
        [int]$width,
        [int]$height
    )
    
    $destBitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($destBitmap)
    
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    
    $graphics.DrawImage($sourceBitmap, 0, 0, $width, $height)
    $graphics.Dispose()
    
    return $destBitmap
}

function Save-PNG {
    param(
        [System.Drawing.Bitmap]$bitmap,
        [string]$filename
    )
    
    try {
        $bitmap.Save($filename, [System.Drawing.Imaging.ImageFormat]::Png)
        $fileSize = (Get-Item $filename).Length
        Write-Host "  Generated: $filename ($fileSize bytes)" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "  Failed: $filename - $_" -ForegroundColor Red
        return $false
    }
}

Write-Host "Extracting source image..." -ForegroundColor Cyan
$sourceBitmap = Get-LargestIconImage -icon $icon
Write-Host "Source image size: $($sourceBitmap.Width)x$($sourceBitmap.Height)" -ForegroundColor Green

$pngSizes = @(
    @{Name="32x32.png"; Width=32; Height=32},
    @{Name="128x128.png"; Width=128; Height=128},
    @{Name="128x128@2x.png"; Width=256; Height=256},
    @{Name="icon.png"; Width=512; Height=512}
)

Write-Host "Generating PNG files..." -ForegroundColor Cyan
$successCount = 0

foreach ($size in $pngSizes) {
    $resized = Resize-Image -sourceBitmap $sourceBitmap -width $size.Width -height $size.Height
    if (Save-PNG -bitmap $resized -filename (Join-Path $outputDir $size.Name)) {
        $successCount++
    }
    $resized.Dispose()
}

$sourceBitmap.Dispose()
$icon.Dispose()

Write-Host "Complete! Generated $successCount/$($pngSizes.Count) PNG files" -ForegroundColor Cyan
Write-Host "Note: macOS .icns file needs to be generated on macOS or use online tools" -ForegroundColor Yellow
Write-Host "All icon files generated successfully!" -ForegroundColor Green