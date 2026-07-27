<#
.SYNOPSIS  تست دودی خودکار: نصب بی‌صدا → راستی‌آزمایی فایل‌ها → حذف بی‌صدا.
.DESCRIPTION
  هیچ نصب‌کننده‌ای نباید منتشر شود که خود CI آن را نصب و حذف نکرده باشد.
  معادل مرحلهٔ «Verify release signature / verify APK» در پایپ‌لاین اندروید.
#>
param(
  [Parameter(Mandatory=$true)][string]$Arch,
  [Parameter(Mandatory=$true)][string]$Version
)
$ErrorActionPreference = 'Stop'
$root  = Split-Path -Parent $PSScriptRoot
$setup = Join-Path $root "dist/Aether-Setup-$Version-$Arch.exe"
if (-not (Test-Path $setup)) { throw "Installer not found: $setup" }

$target = Join-Path $env:TEMP "AetherSmoke-$Arch"
Remove-Item $target -Recurse -Force -ErrorAction SilentlyContinue

# هیچ مرحله‌ای حق ندارد بی‌نهایت منتظر بماند. اگر فرآیندی از سقف
# زمانی بگذرد (مثلاً چون پنجره‌ای نامرئی منتظر کلیک مانده)،
# همان‌جا کشته و با پیام روشن شکست اعلام می‌شود.
# هشدار مهم برای آینده: اسم پارامتر هرگز نباید `$Args` باشد!
# `$Args` در PowerShell یک متغیر خودکار رزروشده است و مقدارش داخل
# تابع خالی می‌شود. نتیجهٔ اجرای قبلی: نصب‌کننده بدون سوییچ‌های
# بی‌صدا اجرا شد، ویزارد گرافیکی روی دسکتاپ نامرئی CI باز ماند
# و تا سقف زمانی منتظر کلیک ماند.
function Invoke-WithTimeout {
  param(
    [string]$Exe,
    [string[]]$ArgumentList,
    [int]$TimeoutSec,
    [string]$What,
    [string]$LogFile = ''
  )
  Write-Host "==> $What : `"$Exe`" $($ArgumentList -join ' ')"
  if (-not $ArgumentList -or $ArgumentList.Count -eq 0) {
    throw "$What was about to run WITHOUT arguments - refusing (this would open an interactive wizard on CI)."
  }
  $p = Start-Process -FilePath $Exe -ArgumentList $ArgumentList -PassThru
  if (-not $p.WaitForExit($TimeoutSec * 1000)) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    if ($LogFile -and (Test-Path $LogFile)) {
      Write-Host "---- last lines of $LogFile ----"
      Get-Content $LogFile -Tail 60 | Write-Host
    }
    throw "$What did not finish within $TimeoutSec seconds - it was probably waiting on a hidden dialog. Killed."
  }
  return $p.ExitCode
}

Write-Host '==> Silent install'
$installLog = Join-Path $env:TEMP 'aether-install.log'
$code = Invoke-WithTimeout -Exe $setup -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/ALLUSERS',"/DIR=`"$target`"","/LOG=`"$installLog`"") -TimeoutSec 300 -What 'Installer' -LogFile $installLog
if ($code -ne 0) {
  if (Test-Path $installLog) { Get-Content $installLog -Tail 60 }
  throw "Installer exited with $code"
}

foreach ($f in @('Aether.exe','engine\aether.exe','engine\wintun.dll','unins000.exe')) {
  if (-not (Test-Path (Join-Path $target $f))) { throw "Installed tree is missing: $f" }
}
Write-Host '==> Installed tree OK (Uninstaller present)'

# مطمئن می‌شویم موتور واقعاً اجراشدنی است (معماری درست، CRT استاتیک).
# هر هسته‌ای پرچم --version را پشتیبانی نمی‌کند، پس این بررسی فقط
# گزارشی است. دروازهٔ واقعی، وجود فایل‌های بالاست.
$engine = Join-Path $target 'engine\aether.exe'
try {
  & $engine --version 2>&1 | Write-Host
  Write-Host "==> engine exit code: $LASTEXITCODE"
} catch {
  Write-Host "::warning::Could not run engine --version: $_"
}

Write-Host '==> Silent uninstall'
$ucode = Invoke-WithTimeout -Exe (Join-Path $target 'unins000.exe') -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -TimeoutSec 180 -What 'Uninstaller'
if ($ucode -ne 0) { throw "Uninstaller exited with $ucode" }
Start-Sleep -Seconds 3
if (Test-Path (Join-Path $target 'Aether.exe')) { throw 'Uninstall left the application behind' }

Write-Host '==> Smoke test passed'
