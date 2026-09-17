<#
.SYNOPSIS
  Launch odyssey1e in dev mode.

.DESCRIPTION
  Wraps `npm run tauri dev` with the two things that waste time otherwise:

    * A previous app window holds the .exe, so a rebuild fails with
      "Access is denied. (os error 5)". This kills it first.
    * A fresh clone has no node_modules, and the failure that causes
      does not say "run npm install".

.PARAMETER Test
  Run the test suite instead of the app.

.PARAMETER Build
  Produce a release build and installer instead of running dev.

.EXAMPLE
  .\run.ps1
  .\run.ps1 -Test
  .\run.ps1 -Build
#>

[CmdletBinding()]
param(
  [switch]$Test,
  [switch]$Build
)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
Set-Location $root

function Require-Command($name, $hint) {
  if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
    Write-Host "  $name not found. $hint" -ForegroundColor Red
    exit 1
  }
}

Write-Host ""
Write-Host "  odyssey1e" -ForegroundColor Cyan
Write-Host ""

Require-Command node  "Install Node LTS from nodejs.org, then open a new terminal."
Require-Command cargo "Install Rust from https://rustup.rs, then open a new terminal."

# A running instance holds target\debug\odyssey1e.exe and any rebuild
# fails on it. The error names a file permission, which sends you looking
# in the wrong place entirely.
$running = Get-Process odyssey1e -ErrorAction SilentlyContinue
if ($running) {
  Write-Host "  closing a running instance..." -ForegroundColor DarkGray
  $running | Stop-Process -Force
  Start-Sleep -Milliseconds 400
}

if (-not (Test-Path (Join-Path $root 'node_modules'))) {
  Write-Host "  installing npm packages (first run only)..." -ForegroundColor DarkGray
  npm install
}

if ($Test) {
  Write-Host "  running tests..." -ForegroundColor DarkGray
  Write-Host ""
  Push-Location (Join-Path $root 'src-tauri')
  try { cargo test } finally { Pop-Location }
  exit $LASTEXITCODE
}

if ($Build) {
  Write-Host "  release build - this takes a while." -ForegroundColor DarkGray
  Write-Host "  output lands in src-tauri\target\release\" -ForegroundColor DarkGray
  Write-Host ""
  npm run tauri build
  exit $LASTEXITCODE
}

# The first dev run after a clean checkout compiles the whole Tauri
# dependency tree. It looks hung for ten to twenty minutes and is not.
if (-not (Test-Path (Join-Path $root 'src-tauri\target\debug'))) {
  Write-Host "  First build - this compiles the whole Tauri tree." -ForegroundColor Yellow
  Write-Host "  Ten to twenty minutes, mostly silent. It is not hung." -ForegroundColor Yellow
  Write-Host ""
}

npm run tauri dev
