<#
.SYNOPSIS
    Builds a single self-contained portfolio executable for Windows.

.DESCRIPTION
    Bundles the two web apps into two HTML documents (web\build.ts, run by
    build.rs), then compiles the Rust binary in release mode. Both documents,
    their inlined stylesheets and scripts, and every template are embedded.
    The public fonts and licenses are embedded too; no extra files are needed.

    One process serves three origins:
        http://127.0.0.1:8050   portfolio.html, the public site
        http://127.0.0.1:8051   admin.html, the dashboard
        http://127.0.0.1:8052   backend, JSON, contact intake, reply threads

    Output:
        dist\portfolio.exe      all three listeners, one database
        dist\READ-ME-FIRST.txt   how to run and configure it

.EXAMPLE
    .\build.ps1

.EXAMPLE
    .\build.ps1 -Run
    Build, then start the servers immediately:
        http://127.0.0.1:8050   portfolio
        http://127.0.0.1:8051   dashboard
        http://127.0.0.1:8052   backend

.EXAMPLE
    .\build.ps1 -SkipTests -OutDir C:\builds
#>

[CmdletBinding()]
param(
    # Directory that receives the built executable.
    [string]$OutDir = '',

    # Skip `cargo test` (the default runs it, because a broken build is worse than a slow one).
    [switch]$SkipTests,

    # Start the compiled server after a successful build.
    [switch]$Run,

    # Keep the console open after a -Run session ends.
    [switch]$NoPause
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$ProjectRoot = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
if (-not $OutDir) { $OutDir = Join-Path $ProjectRoot 'dist' }
$BinaryName  = 'portfolio'
$WebAssets   = @(
    'web\build.ts',
    'web\portfolio.html',
    'web\admin.html',
    'static\base.css',
    'static\site.css',
    'static\detail.css',
    'static\fonts\dm-sans-latin.woff2',
    'static\fonts\instrument-serif-latin.woff2',
    'static\fonts\instrument-serif-italic-latin.woff2',
    'static\fonts\DM-Sans-OFL.txt',
    'static\fonts\Instrument-Serif-OFL.txt',
    'static\admin.css',
    'static\core.js',
    'static\site.js',
    'static\icon.svg',
    'static\gideon-sigilai.svg',
    'static\gideon-sigilai.png',
    'templates'
)
$SiteUrl  = 'http://127.0.0.1:8050'
$AdminUrl = 'http://127.0.0.1:8051'
$ApiUrl   = 'http://127.0.0.1:8052'

function Write-Step {
    param([string]$Message)
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Message)
    Write-Host "    $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "    $Message" -ForegroundColor Yellow
}

function Format-Size {
    param([long]$Bytes)
    if ($Bytes -ge 1MB) { return ('{0:N1} MB' -f ($Bytes / 1MB)) }
    return ('{0:N0} KB' -f ($Bytes / 1KB))
}

function Get-RelativePath {
    param([string]$Base, [string]$Path)
    $resolvedBase = (Resolve-Path -LiteralPath $Base).Path.TrimEnd('\')
    $resolvedPath = (Resolve-Path -LiteralPath $Path).Path
    if ($resolvedPath.StartsWith($resolvedBase, [StringComparison]::OrdinalIgnoreCase)) {
        return $resolvedPath.Substring($resolvedBase.Length).TrimStart('\')
    }
    return $resolvedPath
}

# ----------------------------------------------------------------- toolchain

Write-Step 'Checking toolchain'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host '    cargo was not found on PATH.' -ForegroundColor Red
    Write-Host '    Install the Rust toolchain from https://rustup.rs and run this script again.'
    exit 1
}

if (-not (Get-Command bun -ErrorAction SilentlyContinue)) {
    Write-Host '    bun was not found on PATH.' -ForegroundColor Red
    Write-Host '    build.rs runs web\build.ts, which inlines CSS, JS, and the icon into two HTML documents.'
    Write-Host '    Install Bun from https://bun.sh and run this script again.'
    exit 1
}

$cargoVersion = (& cargo --version) -join ' '
$rustcVersion = (& rustc --version) -join ' '
$bunVersion = (& bun --version) -join ' '
Write-Ok $cargoVersion
Write-Ok $rustcVersion
Write-Ok "bun $bunVersion"
Write-Ok 'cargo build runs web\build.ts: one HTML document per web app, stylesheets and scripts inlined.'

Push-Location $ProjectRoot
$exitCode = 0
try {
    # ------------------------------------------------------------ preflight

    Write-Step 'Verifying embedded web assets'

    foreach ($asset in $WebAssets) {
        $full = Join-Path $ProjectRoot $asset
        if (-not (Test-Path -LiteralPath $full)) {
            Write-Host "    Missing web asset: $asset" -ForegroundColor Red
            Write-Host '    The executable embeds these files at compile time, so the build cannot continue.'
            exit 1
        }
    }
    $templateCount = (Get-ChildItem -LiteralPath (Join-Path $ProjectRoot 'templates') -Filter '*.html' -File).Count
    Write-Ok "$($WebAssets.Count - 1) web assets plus $templateCount templates found"

    # --------------------------------------------------------------- tests

    if (-not $SkipTests) {
        Write-Step 'Running unit tests'
        & cargo test --quiet
        if ($LASTEXITCODE -ne 0) {
            throw "cargo test failed with exit code $LASTEXITCODE."
        }
        Write-Ok 'All tests passed'
    }
    else {
        Write-Warn 'Tests skipped (-SkipTests)'
    }

    # --------------------------------------------------------------- build

    Write-Step 'Compiling release build (this can take a few minutes)'
    & cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE."
    }

    $builtBinary = Join-Path $ProjectRoot "target\release\$BinaryName.exe"
    if (-not (Test-Path -LiteralPath $builtBinary)) {
        throw "Expected binary not found: $builtBinary"
    }
    Write-Ok "Compiled $(Format-Size (Get-Item -LiteralPath $builtBinary).Length)"

    # --------------------------------------------------------------- stage

    Write-Step "Staging output in $OutDir"

    if (-not (Test-Path -LiteralPath $OutDir)) {
        New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
    }
    $OutDir = (Resolve-Path -LiteralPath $OutDir).Path

    $targetExe = Join-Path $OutDir "$BinaryName.exe"
    if (Test-Path -LiteralPath $targetExe) {
        Remove-Item -LiteralPath $targetExe -Force
    }
    Copy-Item -LiteralPath $builtBinary -Destination $targetExe
    Write-Ok "$BinaryName.exe -> $targetExe"

    # Nothing else is copied: both HTML documents, their stylesheets, scripts,
    # templates, public fonts, and font licenses are inside the executable.

    # -------------------------------------------------------------- launcher

    $readme = @'
Portfolio server
================

Run
---
  portfolio.exe

One process, three origins:

  http://127.0.0.1:8050   portfolio.html - the public site
  http://127.0.0.1:8051   admin.html     - the dashboard
  http://127.0.0.1:8052   backend        - JSON, contact intake, reply threads

The executable contains both web applications: pages and dashboard, with each
surface's stylesheet and client script inlined into its own document. It needs
no Node.js, no bundler, and no extra files.

The portfolio does not load the dashboard's markup, CSS, or script, and it has
no dashboard routes at all: opening 8050 can only reach the public site. The
backend has no dashboard routes and no write path for content. The dashboard
runs on its own origin; neither of the other two resolves sessions, so its
cookie grants nothing there. Give it its own host name in production and the
cookie stops crossing origins entirely.

First run
---------
The database is created next to the executable (data\portfolio.db) and seeded
with three essays plus the starter portfolio content. Open the dashboard at
http://127.0.0.1:8051; writing, the inbox, and portfolio content are its tabs.

Set an admin password before the first run, or the dashboard stays locked:

  $env:ADMIN_PASSWORD = 'a-long-password-of-your-choosing'
  .\portfolio.exe

The password is hashed with argon2 and stored; the plaintext variable is only
read once, on first boot.

Configuration
-------------
  SITE_BIND         portfolio listen address   default 127.0.0.1:8050
  ADMIN_BIND        dashboard listen address   default 127.0.0.1:8051
  API_BIND          backend listen address     default 127.0.0.1:8052
  BIND              legacy alias for SITE_BIND
  API_URL           backend origin in pages    default http://<API_BIND>
  ADMIN_URL         dashboard origin           default http://<ADMIN_BIND>
  BASE_URL          public URL                 default http://<SITE_BIND>
  DATABASE_PATH     SQLite file                default data/portfolio.db
  ADMIN_USERNAME    dashboard user name        default gideon
  ADMIN_PASSWORD    first-run password         (unset = dashboard locked)
  ADMIN_PASSWORD_HASH  pre-computed argon2 hash, skips ADMIN_PASSWORD
  TRUST_PROXY_IP    honour X-Forwarded-For     (set only behind a trusted proxy)
  RUST_LOG          log level                  default portfolio=info

The three origins must be reachable from the browser at the addresses above;
API_URL is what the portfolio's contact form posts to, and it is named in the
page's Content-Security-Policy.

Exposing it publicly
--------------------
Bind each port to 0.0.0.0 and put a TLS-terminating reverse proxy in front. Give
the dashboard its own host name so the site never shares an origin with it:

  $env:SITE_BIND = '0.0.0.0:8050'
  $env:ADMIN_BIND = '0.0.0.0:8051'
  $env:API_BIND = '0.0.0.0:8052'
  $env:BASE_URL = 'https://your-domain.example'
  $env:API_URL = 'https://api.your-domain.example'
  $env:ADMIN_URL = 'https://admin.your-domain.example'

Sessions use httpOnly, SameSite=Strict cookies. Serve over HTTPS in production.
'@
    $readme | Set-Content -LiteralPath (Join-Path $OutDir 'READ-ME-FIRST.txt') -Encoding UTF8
    Write-Ok 'READ-ME-FIRST.txt written'

    # ---------------------------------------------------------------- report

    Write-Step 'Build artefacts'
    Write-Ok "portfolio  $SiteUrl"
    Write-Ok "dashboard  $AdminUrl"
    Write-Ok "backend    $ApiUrl"
    Get-ChildItem -LiteralPath $OutDir | ForEach-Object {
        $size = if ($_.PSIsContainer) { '<dir>' } else { Format-Size $_.Length }
        Write-Host ("    {0,-22} {1}" -f $_.Name, $size)
    }

    Write-Host ''
    Write-Host "Done. $((Get-Item -LiteralPath $targetExe).Length / 1MB | ForEach-Object { '{0:N1} MB' -f $_ }) single executable serving three origins, in $OutDir" -ForegroundColor Green

    # ------------------------------------------------------------------ run

    if ($Run) {
        Write-Step "Starting $BinaryName.exe (Ctrl+C to stop)"
        Write-Host "    portfolio  $SiteUrl" -ForegroundColor Cyan
        Write-Host "    dashboard  $AdminUrl" -ForegroundColor Cyan
        Write-Host "    backend    $ApiUrl" -ForegroundColor Cyan
        Push-Location $OutDir
        try {
            & $targetExe
            $exitCode = $LASTEXITCODE
        }
        finally {
            Pop-Location
        }
    }
}
catch {
    Write-Host ''
    Write-Host "BUILD FAILED: $($_.Exception.Message)" -ForegroundColor Red
    $exitCode = 1
}
finally {
    Pop-Location
}

exit $exitCode
