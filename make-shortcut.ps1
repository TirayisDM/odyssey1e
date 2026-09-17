<#
.SYNOPSIS
  Put an odyssey1e shortcut on the Desktop.

.DESCRIPTION
  Prefers the release build when one exists - that is a real Windows
  app: no console window, no Node, no Rust. Falls back to a dev-mode
  shortcut that runs run.ps1, which does open a console and does need
  the toolchain.

  Build the real one first if you want the proper thing:
      npm run tauri build

.PARAMETER Dev
  Force the dev shortcut even when a release build exists.
#>

[CmdletBinding()]
param([switch]$Dev)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot

$exe      = Join-Path $root 'src-tauri\target\release\odyssey1e.exe'
$icon     = Join-Path $root 'src-tauri\icons\icon.ico'
$desktop  = [Environment]::GetFolderPath('Desktop')
$linkPath = Join-Path $desktop 'odyssey1e.lnk'

$useExe = (Test-Path $exe) -and (-not $Dev)

$shell = New-Object -ComObject WScript.Shell
$lnk = $shell.CreateShortcut($linkPath)

if ($useExe) {
    $lnk.TargetPath       = $exe
    $lnk.WorkingDirectory = Split-Path $exe
    $lnk.Description      = 'odyssey1e'
    $mode = 'release build'
} else {
    # -ExecutionPolicy Bypass so a machine set to Restricted still runs
    # it; -NoProfile so a slow PowerShell profile does not delay launch.
    $lnk.TargetPath = (Get-Command powershell.exe).Source
    $lnk.Arguments  = "-ExecutionPolicy Bypass -NoProfile -File `"$(Join-Path $root 'run.ps1')`""
    $lnk.WorkingDirectory = $root
    $lnk.Description      = 'odyssey1e (dev - opens a console)'
    $mode = 'dev mode via run.ps1'
}

if (Test-Path $icon) { $lnk.IconLocation = $icon }
$lnk.Save()

Write-Host ""
Write-Host "  Shortcut created on the Desktop - $mode" -ForegroundColor Green
if (-not $useExe) {
    Write-Host "  For a real app with no console, run:  npm run tauri build" -ForegroundColor DarkGray
    Write-Host "  then re-run this script." -ForegroundColor DarkGray
}
Write-Host ""
