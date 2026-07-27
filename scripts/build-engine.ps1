<#
.SYNOPSIS
  کامپایل هستهٔ aether برای ویندوز (دقیقاً همان سورس مخزن اندروید).
.DESCRIPTION
  معادل scripts/build-natives.sh در AetherMobile.
  خروجی: dist-engine/aether.exe

  سخت‌سازی‌های اضافه‌شده تا بیلد بی‌دلیل نشکند:
    - مسیر کریت هسته دیگر ثابت فرض نمی‌شود؛ خودش پیدایش می‌کند.
    - اگر Cargo.lock نباشد، بدون --locked دوباره تلاش می‌کند.
#>
param(
  [Parameter(Mandatory = $true)][string]$Target
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out  = Join-Path $root 'dist-engine'
New-Item -ItemType Directory -Force -Path $out | Out-Null

# ---- پیدا کردن کریت هسته -------------------------------------------
# چیدمان مخزن بالادست ممکن است تغییر کند؛ مسیر ثابت یک بمب ساعتی است.
$candidates = @(
  (Join-Path $root 'native/aether/aether'),
  (Join-Path $root 'native/aether')
)
$core = $null
foreach ($c in $candidates) {
  if (Test-Path (Join-Path $c 'Cargo.toml')) { $core = $c; break }
}
if (-not $core) {
  # آخرین تلاش: دنبال هر Cargo.toml که یک باینری به نام aether می‌سازد.
  $found = Get-ChildItem (Join-Path $root 'native') -Recurse -Filter 'Cargo.toml' -ErrorAction SilentlyContinue |
           Where-Object { (Get-Content $_.FullName -Raw) -match 'name\s*=\s*"aether"' } |
           Select-Object -First 1
  if ($found) { $core = Split-Path -Parent $found.FullName }
}
if (-not $core) {
  throw "Could not locate the aether core crate under $root\native (did sync-core run?)"
}
Write-Host "==> Core crate: $core"
Write-Host "==> Building aether core for $Target"

# static-CRT: خروجی نباید به Visual C++ Redistributable وابسته باشد، وگرنه
# نسخهٔ پرتابل روی بعضی سیستم‌ها اجرا نمی‌شود.
$env:RUSTFLAGS = '-C target-feature=+crt-static'

Push-Location $core
try {
  $useLocked = Test-Path (Join-Path $core 'Cargo.lock')
  if ($useLocked) {
    cargo build --release --locked --target $Target
    if ($LASTEXITCODE -ne 0) {
      Write-Host '::warning::--locked build failed; retrying without it.'
      cargo build --release --target $Target
    }
  } else {
    cargo build --release --target $Target
  }
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

  $bin = Join-Path $core "target/$Target/release/aether.exe"
  if (-not (Test-Path $bin)) {
    # برخی نسخه‌های هسته باینری را با نام دیگری تولید می‌کنند.
    $bin = Get-ChildItem "target/$Target/release/*.exe" -ErrorAction SilentlyContinue |
           Where-Object { $_.Name -notmatch 'build|deps' } |
           Select-Object -First 1 -ExpandProperty FullName
  }
  if (-not $bin) { throw 'aether.exe was not produced' }

  Copy-Item $bin (Join-Path $out 'aether.exe') -Force
  Write-Host "==> engine -> dist-engine/aether.exe"

  # همان کاری که مخزن اندروید می‌کند: نسخهٔ هسته در خروجی ثبت شود
  # تا در پنل About نمایش داده شود.
  # نسخهٔ واقعی هسته را ثبت کن، نه برچسب ثابت مخزن.
  # sync-core نسخهٔ واقعی (مثلاً 1.4.0) را در native/aether/CORE_VERSION می‌نویسد؛
  # فایل ریشهٔ مخزن فقط یک برچسب baseline است و پنل About را گمراه می‌کرد.
  $realVer = Join-Path $root 'native/aether/CORE_VERSION'
  if (Test-Path $realVer) {
    Copy-Item $realVer (Join-Path $out 'CORE_VERSION') -Force
  } else {
    Copy-Item (Join-Path $root 'CORE_VERSION') (Join-Path $out 'CORE_VERSION') -Force -ErrorAction SilentlyContinue
  }
}
finally { Pop-Location }
