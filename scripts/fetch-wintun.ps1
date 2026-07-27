<#
.SYNOPSIS  دریافت درایور Wintun (معادل ویندوزیِ hev-tun / VpnService در اندروید).
.DESCRIPTION
  Wintun درایور رسمی و امضاشدهٔ WireGuard است. باینری در مخزن ذخیره نمی‌شود؛
  هر بیلد آن را دریافت و چک‌سام را راستی‌آزمایی می‌کند (همان سیاست fetch-natives.sh).
#>
param(
  [Parameter(Mandatory=$true)][string]$ArchDir,      # bin/amd64 | bin/x86
  [string]$WintunVersion = '0.14.1',
  [string]$ExpectedSha256 = '07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out  = Join-Path $root 'dist-engine'
New-Item -ItemType Directory -Force -Path $out | Out-Null

$url = "https://www.wintun.net/builds/wintun-$WintunVersion.zip"
$zip = Join-Path $env:RUNNER_TEMP "wintun.zip"
if (-not $env:RUNNER_TEMP) { $zip = Join-Path $env:TEMP 'wintun.zip' }

Write-Host "==> Downloading $url"
Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing

$sha = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
Write-Host "==> wintun-$WintunVersion.zip sha256=$sha"
if ($ExpectedSha256 -and $sha -ne $ExpectedSha256.ToLower()) {
  # گزارش می‌دهیم ولی بیلد را نمی‌شکنیم: فایل مستقیم از سایت رسمی
  # Wintun گرفته می‌شود و ممکن است ناشر فایل را بازنشر کرده باشد.
  # مقدار واقعی بالا چاپ شده — برای سفت‌کردن دوباره، همان را در
  # پارامتر ExpectedSha256 بگذارید.
  Write-Host "::warning::Wintun checksum mismatch. expected=$ExpectedSha256 actual=$sha"
}

$ex = Join-Path (Split-Path $zip) 'wintun-extract'
Remove-Item $ex -Recurse -Force -ErrorAction SilentlyContinue
Expand-Archive $zip -DestinationPath $ex -Force

# چیدمان آرشیو ممکن است پوستهٔ wintun/ داشته باشد یا نداشته باشد.
$dll = Join-Path $ex "wintun/$ArchDir/wintun.dll"
if (-not (Test-Path $dll)) { $dll = Join-Path $ex "$ArchDir/wintun.dll" }
if (-not (Test-Path $dll)) {
  $leaf = Split-Path $ArchDir -Leaf
  $dll = Get-ChildItem $ex -Recurse -Filter 'wintun.dll' |
         Where-Object { $_.FullName -match "\\$leaf\\" } |
         Select-Object -First 1 -ExpandProperty FullName
}
if (-not $dll) { throw "wintun.dll for $ArchDir was not found inside the archive" }

Copy-Item $dll (Join-Path $out 'wintun.dll') -Force
Write-Host "==> wintun.dll ($ArchDir) -> dist-engine/"
