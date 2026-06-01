$root = 'c:\Users\P16V\Desktop\个人开发\clashverge-clean'
$exts = @('.rs','.go','.ts','.tsx','.js','.mjs','.css','.html','.json','.yaml','.yml','.toml','.py','.sh')
$exclude = @('pnpm-lock.yaml','Cargo.lock','package-lock.json')
$stats = @{}
Get-ChildItem -Recurse -File -Path $root | Where-Object {
    $_.FullName -notmatch '\\.git\\' -and
    $_.FullName -notmatch '\\node_modules\\' -and
    $_.FullName -notmatch '\\target\\' -and
    $_.FullName -notmatch '\\dist\\' -and
    $_.FullName -notmatch '\\dist-js\\' -and
    $exts -contains $_.Extension -and
    $exclude -notcontains $_.Name
} | ForEach-Object {
    $ext = $_.Extension
    $lines = 0
    try { $lines = (Get-Content $_.FullName -ErrorAction SilentlyContinue | Measure-Object -Line).Lines } catch {}
    if (-not $stats.ContainsKey($ext)) { $stats[$ext] = @{Files=0; Lines=0} }
    $stats[$ext].Files += 1
    $stats[$ext].Lines += $lines
}
$stats.GetEnumerator() | Sort-Object { $_.Value.Lines } -Descending | ForEach-Object {
    Write-Output ("{0,-8} {1,6} files  {2,8} lines" -f $_.Key, $_.Value.Files, $_.Value.Lines)
}
$totalFiles = ($stats.Values | Measure-Object -Property Files -Sum).Sum
$totalLines = ($stats.Values | Measure-Object -Property Lines -Sum).Sum
Write-Output ("{0,-8} {1,6} files  {2,8} lines" -f "TOTAL", $totalFiles, $totalLines)
