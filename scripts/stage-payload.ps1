<#
.SYNOPSIS  جمع‌آوری خروجی نهایی در dist/payload-<arch> (ورودی مشترک Setup و Portable).
#>
param(
  [Parameter(Mandatory=$true)][string]$Target,
  [Parameter(Mandatory=$true)][string]$Arch
)
$ErrorActionPreference = 'Stop'
$root    = Split-Path -Parent $PSScriptRoot
$relDir  = Join-Path $root "src-tauri/target/$Target/release"
$out     = Join-Path $root "dist/payload-$Arch"

# پیدا کردن اجرایی برنامه.
#
# مهم: وقتی Tauri را با `--bundles none` می‌سازیم، نام خروجی همان نام
# کریت در Cargo.toml است (aether-desktop.exe) نه productName (Aether.exe).
# قبلاً مستقیم Aether.exe را می‌خواستیم — یعنی یک خطای حتمی در مراحل بعد.
# حالا هر دو نام را می‌پذیریم و خروجی را به Aether.exe تغییر نام می‌دهیم.
$bin = $null
foreach ($name in @('Aether.exe', 'aether-desktop.exe', 'aether_desktop.exe')) {
  $candidate = Join-Path $relDir $name
  if (Test-Path $candidate) { $bin = $candidate; break }
}
if (-not $bin) {
  # آخرین تلاش: هر exe سطح بالا که ابزار کمکی build/deps نباشد.
  $bin = Get-ChildItem (Join-Path $relDir '*.exe') -ErrorAction SilentlyContinue |
         Where-Object { $_.Name -notmatch '^(build|deps)' } |
         Sort-Object Length -Descending |
         Select-Object -First 1 -ExpandProperty FullName
}
if (-not $bin) {
  Write-Host "Contents of $relDir :"
  Get-ChildItem $relDir -ErrorAction SilentlyContinue | Select-Object Name, Length | Format-Table -AutoSize
  throw "App binary not found in $relDir"
}
Write-Host "==> App binary: $bin"

Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path (Join-Path $out 'engine') | Out-Null

# همیشه به نام Aether.exe کپی می‌شود — نصب‌کننده، میانبرها و
# قاعدهٔ فایروال همه به همین نام تکیه دارند.
Copy-Item $bin (Join-Path $out 'Aether.exe') -Force
Copy-Item (Join-Path $root 'dist-engine/aether.exe')  (Join-Path $out 'engine') -Force
Copy-Item (Join-Path $root 'dist-engine/wintun.dll')  (Join-Path $out 'engine') -Force
# نسخهٔ واقعی هستهٔ سینک‌شده را در payload بگذار (نه برچسب ثابت ریشهٔ مخزن).
$realVer = Join-Path $root 'native/aether/CORE_VERSION'
if (Test-Path $realVer) { Copy-Item $realVer (Join-Path $out 'engine') -Force }
else { Copy-Item (Join-Path $root 'CORE_VERSION') (Join-Path $out 'engine') -Force }
Copy-Item (Join-Path $root 'LICENSE')                 $out -Force -ErrorAction SilentlyContinue

# دروازهٔ سلامت: هیچ پیلودی بدون موتور و درایور نباید منتشر شود.
foreach ($f in @('Aether.exe','engine/aether.exe','engine/wintun.dll')) {
  if (-not (Test-Path (Join-Path $out $f))) { throw "Payload incomplete: $f is missing" }
}
Get-ChildItem $out -Recurse | Select-Object FullName, Length | Format-Table -AutoSize
Write-Host "==> payload ready: $out"
