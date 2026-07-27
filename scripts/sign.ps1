<#
.SYNOPSIS  امضای کد خروجی‌ها (اختیاری).
.DESCRIPTION
  معادل منطق keystore در مخزن اندروید: امضا باید پایدار بماند.
  اگر اثرانگشت گواهی با مقدار پین‌شده در .github/expected-signer.txt فرق کند،
  بیلد عمداً فیل می‌شود — دقیقاً همان محافظی که در ۱.۲.۲ اندروید اضافه شد.
#>
param([Parameter(Mandatory=$true)][string]$Arch)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

if (-not $env:SIGN_PFX) { Write-Host 'No signing certificate configured - skipping.'; exit 0 }

$pfx = Join-Path $env:RUNNER_TEMP 'code-sign.pfx'
[IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:SIGN_PFX))

$signtool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' |
            Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $signtool) { throw 'signtool.exe not found on the runner' }

$targets = @(
  Join-Path $root "dist/payload-$Arch/Aether.exe"
  Join-Path $root "dist/payload-$Arch/engine/aether.exe"
)
foreach ($t in $targets) {
  if (-not (Test-Path $t)) { continue }
  & $signtool sign /f $pfx /p $env:SIGN_PFX_PASSWORD `
    /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /d 'Aether' $t
  if ($LASTEXITCODE -ne 0) { throw "signtool failed on $t" }
}

# پین کردن اثرانگشت: اگر گواهی عوض شود، بیلد می‌شکند نه کاربر.
$pinFile = Join-Path $root '.github/expected-signer.txt'
$cert = (Get-AuthenticodeSignature (Join-Path $root "dist/payload-$Arch/Aether.exe")).SignerCertificate
$thumb = $cert.Thumbprint.ToUpper()
if (Test-Path $pinFile) {
  $expected = (Get-Content $pinFile | Where-Object { $_ -notmatch '^\s*#' -and $_.Trim() }) -join ''
  if ($expected.Trim().ToUpper() -ne $thumb) {
    throw "Signer changed! expected=$($expected.Trim()) actual=$thumb - refusing to publish."
  }
  Write-Host 'Signer matches the pinned certificate.'
} else {
  "# SHA-1 thumbprint of the certificate that signs every Aether Desktop release.`n$thumb" |
    Set-Content $pinFile -Encoding ascii
  Write-Host "Pinned signer thumbprint: $thumb"
}

Remove-Item $pfx -Force
