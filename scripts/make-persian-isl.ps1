<#
.SYNOPSIS  ساخت فایل زبان فارسی ویزارد نصب.

.DESCRIPTION
  مشکلی که این اسکریپت ریشه‌ای حل می‌کند:

  Inno Setup وقتی یک فایل زبان (.isl) می‌گیرد، توقع دارد همهٔ صدها
  پیام استاندارد در آن باشد. اگر حتی یک پیام جا بیفتد، کامپایلر با
  خطای «The [Messages] section entry … was not found» متوقف می‌شود.
  نوشتن دستی چنین فایلی عملاً بمب ساعتی است.

  راه‌حل: فایل رسمی و کاملِ Default.isl خود Inno را برمی‌داریم، فقط
  پیام‌هایی را که کاربر واقعاً می‌بیند فارسی می‌کنیم، و زبان را
  راست‌به‌چپ می‌کنیم. بقیهٔ پیام‌ها قطعاً سر جایشان هستند.
#>
param(
  [Parameter(Mandatory = $true)][string]$InnoDir
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$src  = Join-Path $InnoDir 'Default.isl'
$dst  = Join-Path $root 'installer/Persian.isl'

if (-not (Test-Path $src)) { throw "Default.isl not found in $InnoDir" }

# Default.isl با کدپیج ویندوزی ذخیره شده؛ UTF-8 می‌خوانیم و UTF-8 می‌نویسیم.
$text = Get-Content $src -Raw -Encoding UTF8

# ---- 1) معرفی زبان -------------------------------------------------
$text = $text -replace '(?m)^LanguageName=.*$',     'LanguageName=<0641><0627><0631><0633><06CC>'
$text = $text -replace '(?m)^LanguageID=.*$',       'LanguageID=$0429'
$text = $text -replace '(?m)^LanguageCodePage=.*$', 'LanguageCodePage=0'
$text = $text -replace '(?m)^DialogFontName=.*$',   'DialogFontName=Segoe UI'
$text = $text -replace '(?m)^WelcomeFontName=.*$',  'WelcomeFontName=Segoe UI'
$text = $text -replace '(?m)^TitleFontName=.*$',    'TitleFontName=Segoe UI'
$text = $text -replace '(?m)^CopyrightFontName=.*$','CopyrightFontName=Segoe UI'

# RightToLeft در Default.isl وجود ندارد؛ به [LangOptions] اضافه‌اش می‌کنیم.
if ($text -notmatch '(?m)^RightToLeft=') {
  $text = $text -replace '(?m)^(LanguageCodePage=.*)$', "`$1`r`nRightToLeft=yes"
}

# ---- 2) فارسی‌کردن پیام‌هایی که کاربر می‌بیند -----------------
# کلیدهایی که در Default.isl وجود نداشته باشند بی‌سروصدا رد می‌شوند،
# پس این مرحله هرگز نمی‌تواند بیلد را بشکند.
$fa = [ordered]@{
  SetupAppTitle          = 'نصب‌کننده'
  SetupWindowTitle       = 'نصب %1'
  UninstallAppTitle      = 'حذف نصب'
  UninstallAppFullTitle  = 'حذف %1'
  ButtonBack             = '< قبلی'
  ButtonNext             = 'بعدی >'
  ButtonInstall          = 'نصب'
  ButtonCancel           = 'انصراف'
  ButtonYes              = 'بله'
  ButtonNo               = 'خیر'
  ButtonFinish           = 'پایان'
  ButtonBrowse           = 'مرور…'
  ExitSetupTitle         = 'خروج از نصب'
  WelcomeLabel1          = 'خوش آمدید به نصب [name]'
  WelcomeLabel2          = 'این برنامه [name/ver] را روی رایانهٔ شما نصب می‌کند.%n%nپیش از ادامه، بهتر است برنامه‌های دیگر را ببندید.'
  WizardLicense          = 'قرارداد لایسنس'
  LicenseLabel           = 'لطفاً قرارداد زیر را بخوانید.'
  LicenseAccepted        = 'موافقم'
  LicenseNotAccepted     = 'موافق نیستم'
  WizardSelectDir        = 'انتخاب محل نصب'
  SelectDirDesc          = '[name] کجا نصب شود؟'
  SelectDirLabel3        = 'برنامه در پوشهٔ زیر نصب می‌شود.'
  WizardSelectTasks      = 'انتخاب کارهای اضافی'
  SelectTasksDesc        = 'چه کارهای دیگری انجام شود؟'
  WizardReady            = 'آمادهٔ نصب'
  ReadyLabel1            = 'نصب‌کننده آماده است [name] را نصب کند.'
  WizardInstalling       = 'در حال نصب'
  InstallingLabel        = 'لطفاً منتظر بمانید…'
  FinishedHeadingLabel   = 'نصب [name] کامل شد'
  FinishedLabel          = '[name] روی رایانهٔ شما نصب شد.'
  ClickFinish            = 'برای بستن نصب‌کننده روی پایان کلیک کنید.'
  CreateDesktopIcon      = 'ایجاد میانبر روی دسکتاپ'
  AdditionalIcons        = 'میانبرها:'
  ConfirmUninstall       = 'مطمئنید می‌خواهید %1 و همهٔ اجزای آن حذف شوند؟'
  UninstallStatusLabel   = 'در حال حذف %1…'
}

$applied = 0
foreach ($key in $fa.Keys) {
  $pattern = "(?m)^$([regex]::Escape($key))=.*$"
  if ($text -match $pattern) {
    $text = [regex]::Replace($text, $pattern, "$key=$($fa[$key])")
    $applied++
  }
}

New-Item -ItemType Directory -Force -Path (Split-Path $dst) | Out-Null
[IO.File]::WriteAllText($dst, $text, (New-Object Text.UTF8Encoding $true))

Write-Host "==> installer/Persian.isl generated from Default.isl ($applied strings localised)"
