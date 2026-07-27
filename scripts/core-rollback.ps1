<#
.SYNOPSIS  بازگرداندن هسته به نسخهٔ قبلی.
.DESCRIPTION
  معادل scripts/core-rollback.sh مخزن اندروید.
  قاعده: ارتقای خودکار هسته هرگز نباید بتواند یک انتشار را بشکند.
#>
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$snap = Join-Path $root 'native/.core-prev/aether'
$core = Join-Path $root 'native/aether'

if (-not (Test-Path $snap)) {
  throw 'No core snapshot to roll back to - this is a real build error, not an upgrade failure.'
}

Write-Host '::warning::Rolling the engine back to the previously shipped core.'
Remove-Item $core -Recurse -Force
Copy-Item $snap $core -Recurse -Force

# CORE_VERSION هم باید برگردد — مخزن نباید ارتقایی را ثبت کند که کامپایل نمی‌شود.
$prevVersion = Join-Path $snap 'CORE_VERSION'
if (Test-Path $prevVersion) { Copy-Item $prevVersion (Join-Path $root 'CORE_VERSION') -Force }

Write-Host "==> Rolled back to core $(Get-Content (Join-Path $root 'CORE_VERSION') -Raw)"
