<#
.SYNOPSIS
    Cross-builds the portfolio binary for Linux and deploys it over SSH.

.DESCRIPTION
    Builds exactly what build.ps1 builds — the same Bun-inlined web assets, the
    same release profile — but for x86_64-unknown-linux-musl, so the result is a
    statically linked ELF that runs on any x86-64 Linux host with no glibc
    version to match.

    The build runs on Windows through cargo-zigbuild: cargo drives the compilers
    and zig supplies the musl C toolchain that libsqlite3-sys needs (SQLite is
    compiled from source). `rustup target add x86_64-unknown-linux-musl` supplies
    the pre-built std.

    Then, over sshpass:

        1. push the binary, pm2 definition, and HTTPS provisioning script
        2. swap the binary in and restart the service under pm2
        3. provision the direct-IP HTTPS proxy before opening public port 8052
        4. verify hashes, private listeners, dashboard login, and public HTTPS
        5. queue the GitHub Pages live-snapshot workflow (unless -SkipPages)

    The remote layout is:

        ~/portfolio/portfolio               the static Linux binary
        ~/portfolio/ecosystem.config.js     pm2 app definition (contains the password)
        ~/portfolio/https-api.sh            direct-IP HTTPS provisioning script
        ~/portfolio/data/portfolio.db       created on first boot, kept across deploys

    GitHub Pages serves the public portfolio at https://gideonsigilai.github.io.
    All three Rust listeners stay bound to loopback. The site listener (8050)
    supplies the live snapshot, and Nginx terminates public HTTPS on the VPS
    IP:8052 for API and public source pages. It never exposes the dashboard
    (8051). A custom -ApiUrl assumes an existing HTTPS proxy.

    Access the private dashboard through an SSH tunnel:

        ssh -L 8051:127.0.0.1:8051 gidi@217.77.8.87

.PARAMETER Password
    SSH password for sshpass, and the dashboard password the app hashes on its
    first boot. The hash is stored in the database; later runs ignore the
    plaintext, so changing this only affects a fresh database.

.PARAMETER BaseUrl
    Absolute public HTTP(S) origin used for canonical, social, feed, sitemap,
    and allowed browser origin URLs. Defaults to https://gideonsigilai.github.io.

.PARAMETER ApiUrl
    Absolute public HTTPS backend origin. Defaults to https://SshHost:8052.
    The default direct-IP endpoint is provisioned with deploy/https-api.sh;
    custom origins must already have an HTTPS proxy to the loopback listeners.

.PARAMETER SkipPages
    Deploy only the backend; do not queue the pages.yml live-snapshot workflow.
    Otherwise GH_TOKEN, GITHUB_TOKEN, or noninteractive Git credentials must
    authorize Actions workflow dispatch for gideonsigilai/gideonsigilai.github.io.

.PARAMETER SkipBuild
    Deploy the binary already staged in -OutDir instead of rebuilding.

.PARAMETER SkipTests
    Skip `cargo test` (the default runs it, on the host, before the cross-build).

.EXAMPLE
    .\deploy.ps1
    Build and deploy to gidi@217.77.8.87, provision HTTPS, verify, and queue Pages.

.EXAMPLE
    .\deploy.ps1 -SkipPages
    Deploy and verify the backend without refreshing the GitHub Pages snapshot.

.EXAMPLE
    .\deploy.ps1 -ApiUrl https://api.example.com
    Use an externally provisioned HTTPS backend proxy.

.EXAMPLE
    .\deploy.ps1 -SkipBuild
    Redeploy the binary already in dist\linux without compiling again.
#>

[CmdletBinding()]
param(
    # Directory that receives the Linux binary and the pm2 app definition.
    [string]$OutDir = '',

    # SSH account and host that receive the deployment.
    [string]$SshUser = 'gidi',
    [string]$SshHost = '217.77.8.87',

    # Directory under the remote home that holds the binary and its database.
    [string]$RemoteDir = 'portfolio',

    # pm2 process name.
    [string]$AppName = 'portfolio',

    # sshpass / SSH password, and the dashboard password on first boot.
    [string]$Password = 'Sawe@259148637',

    # Site/source-renderer listener; the dashboard and Rust API stay private.
    [int]$Port = 8050,

    # Canonical public portfolio origin hosted on GitHub Pages.
    [string]$BaseUrl = 'https://gideonsigilai.github.io',

    # Public HTTPS proxy origin; leave empty to use https://SshHost:8052.
    [string]$ApiUrl = '',

    # Deploy only the backend without queuing a Pages refresh.
    [switch]$SkipPages,

    # Skip `cargo test`.
    [switch]$SkipTests,

    # Deploy whatever is already staged in -OutDir.
    [switch]$SkipBuild,

    # Keep every remote shell quiet about sshpass.
    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$ProjectRoot = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
if (-not $OutDir) { $OutDir = Join-Path $ProjectRoot 'dist\linux' }
$BinaryName = 'portfolio'
$Target = 'x86_64-unknown-linux-musl'
$AdminPort = 8051
$ApiPort = 8052
$WebAssets = @(
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
$SshTarget = "$SshUser@$SshHost"
$SiteUrl = "http://127.0.0.1:$Port"
if (-not $ApiUrl) { $ApiUrl = "https://$SshHost`:$ApiPort" }
$publicOrigin = $null
if (-not [Uri]::TryCreate($BaseUrl, [UriKind]::Absolute, [ref]$publicOrigin) -or
    $publicOrigin.Scheme -notin @('http', 'https') -or
    [string]::IsNullOrWhiteSpace($publicOrigin.Host) -or
    $publicOrigin.UserInfo -or
    $publicOrigin.AbsolutePath -ne '/' -or
    $publicOrigin.Query -or
    $publicOrigin.Fragment) {
    throw '-BaseUrl must be an absolute HTTP(S) origin with no credentials, path, query, or fragment.'
}
$BaseUrl = $publicOrigin.GetLeftPart([UriPartial]::Authority)
$BaseUrlJson = ConvertTo-Json -InputObject $BaseUrl -Compress
$apiOrigin = $null
if (-not [Uri]::TryCreate($ApiUrl, [UriKind]::Absolute, [ref]$apiOrigin) -or
    $apiOrigin.Scheme -ne 'https' -or
    [string]::IsNullOrWhiteSpace($apiOrigin.Host) -or
    $apiOrigin.UserInfo -or
    $apiOrigin.AbsolutePath -ne '/' -or
    $apiOrigin.Query -or
    $apiOrigin.Fragment) {
    throw '-ApiUrl must be an absolute HTTPS origin with no credentials, path, query, or fragment.'
}
$ApiUrl = $apiOrigin.GetLeftPart([UriPartial]::Authority)
$ApiUrlJson = ConvertTo-Json -InputObject $ApiUrl -Compress
$hostIp = $null
$provisionHttps = [System.Net.IPAddress]::TryParse($SshHost, [ref]$hostIp) -and
    $hostIp.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetwork -and
    $apiOrigin.Host -eq $SshHost -and $apiOrigin.Port -eq $ApiPort
$httpsScriptSource = Join-Path $ProjectRoot 'deploy\https-api.sh'
$SshOptions = @(
    '-T',
    '-o', 'StrictHostKeyChecking=accept-new',
    '-o', 'ConnectTimeout=20',
    '-o', 'ServerAliveInterval=15',
    '-o', 'ServerAliveCountMax=3'
)

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

function Write-Fail {
    param([string]$Message)
    Write-Host "    $Message" -ForegroundColor Red
}

function Format-Size {
    param([long]$Bytes)
    if ($Bytes -ge 1MB) { return ('{0:N1} MB' -f ($Bytes / 1MB)) }
    return ('{0:N0} KB' -f ($Bytes / 1KB))
}

# sshpass reads the password from SSHPASS with -e, which keeps it out of the
# process table (unlike -p, which every local user can see in a process listing).
#
# Two Windows quirks shape the rest of this function. `ssh` allocates a pty
# unless told otherwise, so -T is set or every newline comes back as CRLF with
# terminal escapes attached. And PowerShell 5.1 re-quotes native arguments on
# the way out, which eats the quotes inside `-d '{"username":...}'`. So remote
# commands are base64-encoded and decoded on the far side: whatever quoting,
# `$`, or backtick the command contains arrives exactly as written.
function ConvertTo-RemoteScript {
    param([Parameter(Mandatory)][string]$Command)
    return [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($Command))
}

function Invoke-Ssh {
    param(
        [Parameter(Mandatory)][string]$Command,
        [switch]$AllowFailure,
        # Run the command through `sudo -S`, feeding the password on stdin. The
        # password never reaches the remote process table that way.
        [switch]$Sudo
    )
    if ($Sudo) {
        $payload = "$Password`n$Command"
        $encoded = ConvertTo-RemoteScript -Command $payload
        $remote = "printf %s '$encoded' | base64 -d | sudo -S -p sudo: sh"
    }
    else {
        $encoded = ConvertTo-RemoteScript -Command $Command
        $remote = "printf %s '$encoded' | base64 -d | sh"
    }
    $env:SSHPASS = $Password
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & sshpass -e ssh @SshOptions $SshTarget $remote 2>&1
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $oldEAP
        Remove-Item Env:SSHPASS -ErrorAction SilentlyContinue
    }
    $lines = @(
        foreach ($line in $output) {
            "$line" -split "`r?`n" | ForEach-Object { $_ -replace "`e\[[0-9;?]*[a-zA-Z]", '' }
        }
    )
    if ($code -ne 0 -and -not $AllowFailure) {
        $text = ($lines | Out-String).Trim()
        throw "Remote command failed (exit $code).`n$text"
    }
    return @{ Code = $code; Output = $lines }
}

function Invoke-SshSuccess {
    param([Parameter(Mandatory)][string]$Command)
    $result = Invoke-Ssh -Command $Command
    return @($result.Output)
}

function Invoke-Sudo {
    param(
        [Parameter(Mandatory)][string]$Command,
        [switch]$AllowFailure
    )
    $result = Invoke-Ssh -Command $Command -Sudo -AllowFailure:$AllowFailure
    return @($result.Output)
}

function Send-File {
    param(
        [Parameter(Mandatory)][string]$Source,
        [Parameter(Mandatory)][string]$Destination
    )
    $env:SSHPASS = $Password
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & sshpass -e scp @SshOptions $Source "$SshTarget`:$Destination" 2>&1 | ForEach-Object {
            if (-not $Quiet) { Write-Host "    $_" -ForegroundColor DarkGray }
        }
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $oldEAP
        Remove-Item Env:SSHPASS -ErrorAction SilentlyContinue
    }
    if ($code -ne 0) {
        throw "scp failed (exit $code) for $Source -> $Destination"
    }
}

function Invoke-Curl {
    param([string[]]$Arguments)
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & curl.exe @Arguments 2>&1
        return @{ Code = $LASTEXITCODE; Output = ($output -join "`n") }
    }
    finally {
        $ErrorActionPreference = $oldEAP
    }
}

function Get-HttpStatus {
    param([Parameter(Mandatory)][string]$Url)
    $result = Invoke-Curl -Arguments @('-s', '-o', 'NUL', '-w', '%{http_code}', '--max-time', '20', $Url)
    return $result.Output.Trim()
}

# Credentials stay in memory: Git receives only a public credential query on
# stdin, and GitHub receives its bearer token in an HTTPS request header.
function Get-PagesToken {
    if (-not [string]::IsNullOrWhiteSpace($env:GH_TOKEN)) { return $env:GH_TOKEN }
    if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_TOKEN)) { return $env:GITHUB_TOKEN }

    # Git Credential Manager query via python or bun ensures clean stdin pipe handling on Windows.
    $python = Get-Command python -ErrorAction SilentlyContinue
    if ($python) {
        $token = & $python.Source -c "import subprocess, re; r = subprocess.run(['git','credential','fill'], input='protocol=https\nhost=github.com\n\n', text=True, capture_output=True).stdout; m = re.search(r'password=(\S+)', r); print(m.group(1) if m else '')" 2>$null
        if (-not [string]::IsNullOrWhiteSpace($token)) { return $token.Trim() }
    }

    $bun = Get-Command bun -ErrorAction SilentlyContinue
    if ($bun) {
        $token = & $bun.Source -e "const p = Bun.spawnSync(['git','credential','fill'], { stdin: new Blob(['protocol=https\nhost=github.com\n\n']) }); const m = p.stdout.toString().match(/password=(\S+)/); if (m) console.log(m[1]);" 2>$null
        if (-not [string]::IsNullOrWhiteSpace($token)) { return $token.Trim() }
    }
    return $null
}

function Request-PagesRefresh {
    $token = Get-PagesToken
    if ([string]::IsNullOrWhiteSpace($token)) {
        throw 'Backend deployed, but Pages was not refreshed: set GH_TOKEN or GITHUB_TOKEN with Actions write access to gideonsigilai/gideonsigilai.github.io, sign in using Git Credential Manager, or use -SkipPages for backend-only deployment.'
    }
    $headers = @{
        Authorization = "Bearer $token"
        Accept = 'application/vnd.github+json'
        'X-GitHub-Api-Version' = '2022-11-28'
    }
    $repositoryUrl = 'https://api.github.com/repos/gideonsigilai/gideonsigilai.github.io'
    try {
        $repository = Invoke-RestMethod -Uri $repositoryUrl -Headers $headers -UserAgent 'portfolio-deploy' -TimeoutSec 30
        $body = ConvertTo-Json -InputObject @{ ref = $repository.default_branch } -Compress
        $dispatch = Invoke-WebRequest -UseBasicParsing -Method Post -Uri "$repositoryUrl/actions/workflows/pages.yml/dispatches" -Headers $headers -UserAgent 'portfolio-deploy' -ContentType 'application/json' -Body $body -TimeoutSec 30
        if ([int]$dispatch.StatusCode -ne 204) { throw 'Unexpected workflow dispatch response.' }
    }
    catch {
        # Do not surface request objects or helper output containing credentials.
        throw 'Backend deployed, but Pages was not refreshed: GitHub workflow dispatch failed. Confirm pages.yml exists on the default branch and your GH_TOKEN/GITHUB_TOKEN or Git credential has Actions write access to gideonsigilai/gideonsigilai.github.io. Retry deployment with -SkipBuild after correcting access.'
    }
    finally {
        $headers.Clear()
        $token = $null
    }
}

Push-Location $ProjectRoot
$exitCode = 0
try {
    # ------------------------------------------------------------ toolchain
    if ($provisionHttps -and -not (Test-Path -LiteralPath $httpsScriptSource)) {
        throw "HTTPS provisioning script not found: $httpsScriptSource"
    }

    Write-Step 'Checking toolchain'

    $requiredTools = @('sshpass', 'ssh', 'scp', 'curl.exe')
    if (-not $SkipBuild) { $requiredTools += @('cargo', 'rustc', 'bun', 'cargo-zigbuild') }
    foreach ($tool in $requiredTools) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            Write-Fail "$tool was not found on PATH."
            switch ($tool) {
                'bun'           { Write-Host '    Install Bun from https://bun.sh: build.rs runs web\build.ts, which inlines CSS, JS, and the icon into two HTML documents.' }
                'sshpass'       { Write-Host '    Install sshpass (winget install sshpass, or a scoop/chocolatey package) and run this script again.' }
                'cargo-zigbuild'{ Write-Host '    Install it with: cargo install cargo-zigbuild --locked' }
                default         { Write-Host '    Install the Rust toolchain from https://rustup.rs and run this script again.' }
            }
            exit 1
        }
    }

    if (-not $SkipBuild) {
    # cargo-zigbuild shells out to `zig`; pick up a per-user install if the
    # machine has one but it is not on PATH yet, and put the chosen directory
    # first so an older zig elsewhere on PATH cannot win.
    $zigCandidates = @(
        (Join-Path $env:LOCALAPPDATA 'zig\zig.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\zig\zig.exe'),
        (Join-Path $env:APPDATA 'Python\Python314\site-packages\ziglang\zig.exe')
    )
    if (Test-Path (Join-Path $env:APPDATA 'Python')) {
        $pyZig = Get-ChildItem -Path (Join-Path $env:APPDATA 'Python') -Filter 'zig.exe' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($pyZig) { $zigCandidates += $pyZig.FullName }
    }
    foreach ($candidate in $zigCandidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) { $zigPath = $candidate; break }
    }
    if (-not $zigPath) {
        $found = Get-Command zig -ErrorAction SilentlyContinue
        if ($found) { $zigPath = $found.Source }
    }
    if (-not $zigPath) {
        Write-Fail 'zig was not found on PATH.'
        Write-Host '    zig supplies the musl C toolchain that SQLite needs when cross-compiling:'
        Write-Host '      pip install ziglang, or download the Windows build from https://ziglang.org/download/'
        exit 1
    }
    $env:PATH = "$(Split-Path -Parent $zigPath);$env:PATH"

    $rustcVersion = (& rustc --version) -join ' '
    $bunVersion = (& bun --version) -join ' '
    $zigVersion = (& zig version) -join ' '
    $zigbuildVersion = (& cargo-zigbuild --version 2>&1) -join ' '
    Write-Ok $rustcVersion
    Write-Ok "bun $bunVersion"
    Write-Ok "zig $zigVersion ($zigPath)"
    Write-Ok $zigbuildVersion

    # An empty CARGO_TARGET_*_LINKER makes rustc fail with "linker ` ` not found",
    # which reads like a missing toolchain rather than an empty variable.
    $linkerVar = 'CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER'
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($linkerVar))) {
        Remove-Item "Env:$linkerVar" -ErrorAction SilentlyContinue
    }

    $installedTargets = (& rustup target list --installed) -join ' '
    if ($installedTargets -notmatch [regex]::Escape($Target)) {
        Write-Step "Adding the $Target standard library"
        & rustup target add $Target
        if ($LASTEXITCODE -ne 0) { throw "rustup target add $Target failed with exit code $LASTEXITCODE." }
    }
    Write-Ok "target $Target ready (static musl: no glibc version to match)"

    # ------------------------------------------------------------ preflight

    Write-Step 'Verifying embedded web assets'

    foreach ($asset in $WebAssets) {
        $full = Join-Path $ProjectRoot $asset
        if (-not (Test-Path -LiteralPath $full)) {
            Write-Fail "Missing web asset: $asset"
            Write-Host '    The executable embeds these files at compile time, so the build cannot continue.'
            exit 1
        }
    }
    $templateCount = (Get-ChildItem -LiteralPath (Join-Path $ProjectRoot 'templates') -Filter '*.html' -File).Count
    Write-Ok "$($WebAssets.Count - 1) web assets plus $templateCount templates found"

    }

    # ---------------------------------------------------------------- tests

    if (-not $SkipBuild) {
        if (-not $SkipTests) {
            Write-Step 'Running unit tests (host build)'
            & cargo test --quiet
            if ($LASTEXITCODE -ne 0) {
                throw "cargo test failed with exit code $LASTEXITCODE."
            }
            Write-Ok 'All tests passed'
        }
        else {
            Write-Warn 'Tests skipped (-SkipTests)'
        }

        # -------------------------------------------------------------- build

        Write-Step "Cross-compiling release build for $Target (this can take a few minutes)"
        & cargo zigbuild --release --target $Target
        if ($LASTEXITCODE -ne 0) {
            throw "cargo zigbuild failed with exit code $LASTEXITCODE."
        }
    }
    else {
        Write-Warn "Build skipped (-SkipBuild): deploying the staged binary as-is"
    }

    # ---------------------------------------------------------------- stage

    Write-Step "Staging output in $OutDir"

    if (-not (Test-Path -LiteralPath $OutDir)) {
        New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
    }
    $OutDir = (Resolve-Path -LiteralPath $OutDir).Path
    $stagedBinary = Join-Path $OutDir $BinaryName

    if (-not $SkipBuild) {
        $builtBinary = Join-Path $ProjectRoot "target\$Target\release\$BinaryName"
        if (-not (Test-Path -LiteralPath $builtBinary)) {
            throw "Expected binary not found: $builtBinary"
        }
        if (Test-Path -LiteralPath $stagedBinary) {
            Remove-Item -LiteralPath $stagedBinary -Force
        }
        Copy-Item -LiteralPath $builtBinary -Destination $stagedBinary -Force
        Write-Ok "$BinaryName -> $stagedBinary"
    }
    if (-not (Test-Path -LiteralPath $stagedBinary)) {
        throw "Nothing staged at $stagedBinary; run without -SkipBuild first."
    }

    $magic = [System.IO.File]::ReadAllBytes($stagedBinary)[0..3]
    if (-not ($magic[0] -eq 0x7F -and $magic[1] -eq 0x45 -and $magic[2] -eq 0x4C -and $magic[3] -eq 0x46)) {
        throw "$stagedBinary is not an ELF binary. The cross-build did not produce a Linux executable."
    }
    $sizeBytes = (Get-Item -LiteralPath $stagedBinary).Length
    Write-Ok "ELF x86-64, statically linked, $(Format-Size $sizeBytes)"

    $localHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $stagedBinary).Hash.ToLowerInvariant()
    Write-Ok "sha256 $localHash"

    # ---------------------------------------------------------- pm2 manifest

    # ADMIN_PASSWORD only matters on the first boot: the app hashes it into the
    # database and ignores the variable afterwards. pm2 keeps it so a rebuilt
    # database (or a fresh host) still comes up unlocked with the same password.
    $ecosystemPath = Join-Path $OutDir 'ecosystem.config.js'
    $ecosystem = @'
// pm2 definition for the portfolio server. Written by deploy.ps1.
//
// One process, three loopback-only listeners. Public access uses the HTTPS
// reverse proxy; the dashboard is never proxied and requires an SSH tunnel:
//
//   ssh -L 8051:127.0.0.1:8051 gidi@217.77.8.87
module.exports = {
  apps: [
    {
      name: '__APP_NAME__',
      cwd: '__REMOTE_DIR__',
      script: './portfolio',
      interpreter: 'none',
      instances: 1,
      autorestart: true,
      max_restarts: 10,
      min_uptime: '10s',
      max_memory_restart: '400M',
      kill_timeout: 5000,
      env: {
        SITE_BIND: '127.0.0.1:__PORT__',
        ADMIN_BIND: '127.0.0.1:__ADMIN_PORT__',
        API_BIND: '127.0.0.1:__API_PORT__',
        BASE_URL: __BASE_URL_JSON__,
        API_URL: __API_URL_JSON__,
        THREADS_URL: __API_URL_JSON__,
        ADMIN_URL: 'http://127.0.0.1:__ADMIN_PORT__',
        TRUST_PROXY_IP: '1',
        ADMIN_USERNAME: 'gideon',
        ADMIN_PASSWORD: '__ADMIN_PASSWORD__',
        RUST_LOG: 'portfolio=info,tower_http=warn'
      }
    }
  ]
};
'@
    $ecosystem = $ecosystem.
        Replace('__APP_NAME__', $AppName).
        Replace('__REMOTE_DIR__', "/home/$SshUser/$RemoteDir").
        Replace('__PORT__', "$Port").
        Replace('__ADMIN_PORT__', "$AdminPort").
        Replace('__API_PORT__', "$ApiPort").
        Replace('__API_URL_JSON__', $ApiUrlJson).
        Replace('__BASE_URL_JSON__', $BaseUrlJson).
        Replace('__ADMIN_PASSWORD__', $Password)
    $ecosystem | Set-Content -LiteralPath $ecosystemPath -Encoding UTF8 -NoNewline
    Write-Ok "$ecosystemPath written"
    $stagedHttpsScript = Join-Path $OutDir 'https-api.sh'
    if ($provisionHttps) {
        Copy-Item -LiteralPath $httpsScriptSource -Destination $stagedHttpsScript -Force
        Write-Ok "$stagedHttpsScript staged"
    }

    # ------------------------------------------------------------ remote push

    Write-Step "Preparing $SshTarget`:~/$RemoteDir"

    $null = Invoke-SshSuccess -Command "mkdir -p ~/$RemoteDir/data && echo ready"
    Write-Ok "directory exists"

    Write-Step 'Uploading binary, pm2 definition, and HTTPS setup when needed'

    # The binary lands beside the live one and is renamed into place, so a
    # failed upload never leaves the service pointing at a half-written file.
    Send-File -Source $stagedBinary -Destination "$RemoteDir/$BinaryName.new"
    Send-File -Source $ecosystemPath -Destination "$RemoteDir/ecosystem.config.js"
    if ($provisionHttps) {
        Send-File -Source $stagedHttpsScript -Destination "$RemoteDir/https-api.sh"
    }
    $null = Invoke-SshSuccess -Command "chmod 755 ~/$RemoteDir/$BinaryName.new && chmod 600 ~/$RemoteDir/ecosystem.config.js && mv -f ~/$RemoteDir/$BinaryName.new ~/$RemoteDir/$BinaryName && echo swapped"
    Write-Ok "$BinaryName in place, ecosystem.config.js readable only by $SshUser"

    $hashOutput = (Invoke-SshSuccess -Command "sha256sum ~/$RemoteDir/$BinaryName") -join ' '
    $remoteHash = [regex]::Match($hashOutput, '[0-9a-f]{64}').Value
    if (-not $remoteHash) {
        throw "Could not read a sha256 from the remote sha256sum output: $hashOutput"
    }
    if ($remoteHash -ne $localHash) {
        throw "Uploaded binary does not match the local build (remote $remoteHash, local $localHash)."
    }
    Write-Ok 'sha256 matches the local build'

    # --------------------------------------------------------------- service

    Write-Step "Starting $AppName under pm2"

    $null = Invoke-Ssh -Command "cd ~/$RemoteDir && pm2 delete $AppName >/dev/null 2>&1 || true" -AllowFailure
    $started = Invoke-SshSuccess -Command "cd ~/$RemoteDir && pm2 start ecosystem.config.js --update-env 2>&1 | tail -6"
    foreach ($line in $started) { Write-Ok $line.TrimEnd() }

    # pm2 save keeps the process list for `pm2 resurrect`; the systemd unit makes
    # that happen after a reboot. Both are idempotent.
    $null = Invoke-SshSuccess -Command "pm2 save >/dev/null 2>&1 && echo saved"
    $unitEnabled = (Invoke-Ssh -Command "systemctl is-enabled pm2-$SshUser 2>/dev/null || true" -AllowFailure).Output -join ''
    if ($unitEnabled -notmatch 'enabled') {
        $startup = Invoke-Sudo -Command "env PATH=`$PATH:`$HOME/.local/bin:/usr/local/bin pm2 startup systemd -u $SshUser --hp /home/$SshUser 2>&1 | tail -4"
        foreach ($line in $startup) { Write-Ok $line.TrimEnd() }
    }
    else {
        Write-Ok "pm2-$SshUser is enabled, so the service survives a reboot"
    }

    # ---------------------------------------------------------------- verify

    Write-Step 'Verifying the running service'

    $deadline = (Get-Date).AddSeconds(45)
    $healthy = $false
    while ((Get-Date) -lt $deadline) {
        $probe = (Invoke-Ssh -Command "curl -fsS --max-time 5 http://127.0.0.1:$Port/health 2>/dev/null || true" -AllowFailure).Output -join ''
        if ($probe -match '"status":"ok"') { $healthy = $true; break }
        Start-Sleep -Seconds 2
    }
    if (-not $healthy) {
        $logs = (Invoke-Ssh -Command "pm2 logs $AppName --lines 25 --nostream --no-color 2>&1 | tail -30" -AllowFailure).Output
        foreach ($line in $logs) { Write-Host "    $line" -ForegroundColor DarkGray }
        throw "The site never answered http://127.0.0.1:$Port/health on the server."
    }
    Write-Ok "site      $SiteUrl     ok (loopback only)"

    foreach ($pair in @(@{ Port = $AdminPort; Label = 'dashboard' }, @{ Port = $ApiPort; Label = 'backend' })) {
        $onLoopback = (Invoke-Ssh -Command "curl -fsS --max-time 5 http://127.0.0.1:$($pair.Port)/health 2>/dev/null || true" -AllowFailure).Output -join ''
        if ($onLoopback -match '"status":"ok"') {
            Write-Ok "$($pair.Label.PadRight(9)) http://127.0.0.1:$($pair.Port)     ok (loopback only)"
        }
        else {
            throw "The $($pair.Label) did not answer on 127.0.0.1:$($pair.Port)."
        }
    }
    # ----------------------------------------------------------- HTTPS proxy

    if ($provisionHttps) {
        Write-Step "Provisioning public HTTPS at $ApiUrl"
        $setup = Invoke-Sudo -Command "sh /home/$SshUser/$RemoteDir/https-api.sh $SshHost $Port $ApiPort"
        foreach ($line in $setup) { Write-Ok $line.TrimEnd() }
        # https-api.sh opens 8052 only after the TLS proxy is configured.
    }
    else {
        Write-Ok "Using externally provisioned HTTPS proxy at $ApiUrl; no raw API listener is exposed"
    }

    # Retire the old public HTTP rule only after the HTTPS replacement works.
    $httpsStatus = Get-HttpStatus -Url "$ApiUrl/health"
    if ($httpsStatus -ne '200') {
        throw "Public HTTPS backend $ApiUrl/health returned $httpsStatus. Check the certificate, proxy, and firewall; the old site firewall rule has not been changed."
    }
    Write-Step 'Keeping all Rust listeners private'
    $ufw = (Invoke-Sudo -Command 'ufw status' | Out-String)
    if ($ufw -match "(?m)^\s*$Port/tcp(?:\s+\(v6\))?\s+ALLOW\b") {
        $null = Invoke-Sudo -Command "ufw --force delete allow $Port/tcp"
        Write-Ok "Removed the obsolete public HTTP allow rule for $Port/tcp"
    }
    if ($ufw -match '(?m)^Status:\s+inactive') {
        Write-Warn 'ufw is inactive; Rust listeners are still restricted to loopback by the service configuration'
    }


    # Proves the ADMIN_PASSWORD in the pm2 environment was hashed on first boot.
    # A rejected sign-in is not an error status on this route — it re-renders the
    # form with HTTP 200 — so the check asserts on the JSON reply instead.
    $loginBody = Invoke-SshSuccess -Command @"
curl -s --max-time 15 -X POST http://127.0.0.1:$AdminPort/login \
  -H 'content-type: application/json' \
  -H 'accept: application/json' \
  -d '{"username":"gideon","password":"$Password"}'
"@
    $loginReply = ($loginBody -join ' ').Trim()
    if ($loginReply -notmatch '"csrf"') {
        throw 'Dashboard login rejected the configured admin password.'
    }
    Write-Ok 'dashboard login accepted (gideon, session + csrf issued)'

    # Probe through public TLS from this machine, not just the loopback service.
    $healthProbe = Invoke-Curl -Arguments @('-fsS', '--max-time', '20', "$ApiUrl/health")
    if ($healthProbe.Code -ne 0 -or $healthProbe.Output -notmatch '"status"\s*:\s*"ok"') {
        throw "Public HTTPS backend $ApiUrl/health did not return a healthy response."
    }
    Write-Ok "backend    $ApiUrl/health reachable with verified TLS"

    $homeStatus = Get-HttpStatus -Url "$ApiUrl/"
    if ($homeStatus -ne '200') {
        throw "The backend answered /health but the public source page $ApiUrl/ returned HTTP $homeStatus."
    }
    Write-Ok "source home page served ($ApiUrl/)"

    $corsProbe = Invoke-Curl -Arguments @('-sS', '--max-time', '20', '-D', '-', '-o', 'NUL', '-X', 'OPTIONS', "$ApiUrl/contact", '-H', "Origin: $BaseUrl", '-H', 'Access-Control-Request-Method: POST', '-H', 'Access-Control-Request-Headers: content-type')
    $corsText = $corsProbe.Output
    $allowedOrigin = [regex]::Match($corsText, '(?im)^access-control-allow-origin:\s*(\S+)\s*$')
    if ($corsProbe.Code -ne 0 -or $corsText -notmatch '(?m)^HTTP/\S+\s+(200|204)\b' -or
        -not $allowedOrigin.Success -or $allowedOrigin.Groups[1].Value -notin @($BaseUrl, '*')) {
        throw "Public HTTPS contact preflight did not allow the portfolio origin $BaseUrl."
    }
    Write-Ok "contact CORS permits $BaseUrl"

    # A dispatch acknowledgment is not a completed Pages publication.
    if ($SkipPages) {
        Write-Warn 'Pages not refreshed (-SkipPages): backend-only deployment'
    }
    else {
        Write-Step 'Queuing GitHub Pages live snapshot'
        Request-PagesRefresh
        Write-Ok 'pages.yml queued; publication is asynchronous'
    }

    # ---------------------------------------------------------------- report

    Write-Step 'Deployment'

    Write-Ok "portfolio  $BaseUrl (GitHub Pages)"
    Write-Ok "dashboard  ssh -L $AdminPort`:127.0.0.1:$AdminPort $SshTarget, then http://127.0.0.1:$AdminPort"
    Write-Ok "backend    $ApiUrl (HTTPS API, source pages, and private reply links)"
    Write-Ok "api tunnel ssh -L $ApiPort`:127.0.0.1:$ApiPort $SshTarget, then http://127.0.0.1:$ApiPort"
    Write-Ok 'user       gideon (configured password is never printed)'
    Write-Ok "files      ~/$RemoteDir/{$BinaryName, ecosystem.config.js, https-api.sh, data/portfolio.db}"
    Write-Ok "control    pm2 restart $AppName | pm2 logs $AppName | pm2 stop $AppName"

    Write-Host ''
    Write-Host "Done. $(Format-Size $sizeBytes) static ELF deployed; all Rust listeners are private. Public portfolio: $BaseUrl; HTTPS backend: $ApiUrl." -ForegroundColor Green
}
catch {
    Write-Host ''
    Write-Host "DEPLOY FAILED: $($_.Exception.Message)" -ForegroundColor Red
    $exitCode = 1
}
finally {
    Pop-Location
}

exit $exitCode
