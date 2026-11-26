<#
.SYNOPSIS
    Pushes the local portfolio database to the live server and refreshes GitHub Pages.

.DESCRIPTION
    Uploads data\portfolio.db to the remote portfolio directory, restarts the pm2
    service, and queues the Pages live-snapshot workflow so the public site
    reflects the current database content.
#>

[CmdletBinding()]
param(
    [string]$SshUser = 'gidi',
    [string]$SshHost = '217.77.8.87',
    [string]$Password = 'Sawe@259148637',
    [string]$RemoteDir = 'portfolio',
    [string]$AppName = 'portfolio',
    [string]$LocalDb = '',
    [string]$BaseUrl = 'https://gideonsigilai.github.io',
    [switch]$SkipPages
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$ProjectRoot = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
if (-not $LocalDb) { $LocalDb = Join-Path $ProjectRoot 'data\portfolio.db' }
if (-not (Test-Path -LiteralPath $LocalDb)) { throw "Local database not found: $LocalDb" }

$SshTarget = "$SshUser@$SshHost"
$SshOptions = @(
    '-T',
    '-o', 'StrictHostKeyChecking=accept-new',
    '-o', 'ConnectTimeout=20',
    '-o', 'ServerAliveInterval=15',
    '-o', 'ServerAliveCountMax=3'
)

$env:SSHPASS = $Password
function Invoke-Remote {
    param([string]$Command, [switch]$AllowFailure)
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & sshpass -e ssh @SshOptions $SshTarget $Command 2>&1
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $oldEAP
    }
    if ($code -ne 0 -and -not $AllowFailure) {
        throw "Remote command failed (exit $code): $($output -join ' ')"
    }
    return @{ Code = $code; Output = @($output) }
}

function Send-RemoteFile {
    param([string]$Source, [string]$Destination)
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & sshpass -e scp @SshOptions $Source "$SshTarget`:$Destination" 2>&1 | Out-Null
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $oldEAP
    }
    if ($code -ne 0) { throw "scp failed (exit $code) for $Source -> $Destination" }
}

function Get-PagesToken {
    if (-not [string]::IsNullOrWhiteSpace($env:GH_TOKEN)) { return $env:GH_TOKEN }
    if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_TOKEN)) { return $env:GITHUB_TOKEN }
    $python = Get-Command python -ErrorAction SilentlyContinue
    if ($python) {
        $token = & $python.Source -c "import subprocess, re; r = subprocess.run(['git','credential','fill'], input='protocol=https\nhost=github.com\n\n', text=True, capture_output=True).stdout; m = re.search(r'password=(\S+)', r); print(m.group(1) if m else '')" 2>$null
        if (-not [string]::IsNullOrWhiteSpace($token)) { return $token.Trim() }
    }
    return $null
}

Push-Location $ProjectRoot
try {
    Write-Host "==> Uploading $LocalDb" -ForegroundColor Cyan
    $remoteDb = "$RemoteDir/data/portfolio.db"
    Send-RemoteFile -Source $LocalDb -Destination "$remoteDb.new"
    $null = Invoke-Remote "chmod 600 ~/$remoteDb.new && echo uploaded"

    Write-Host "==> Replacing live database" -ForegroundColor Cyan
    $null = Invoke-Remote "cd ~/$RemoteDir && pm2 stop $AppName >/dev/null 2>&1 || true"
    $null = Invoke-Remote "cd ~/$RemoteDir/data && cp -f portfolio.db portfolio.db.pre-deploy 2>/dev/null || true"
    $null = Invoke-Remote "cd ~/$RemoteDir/data && mv -f portfolio.db.new portfolio.db && chmod 600 portfolio.db && echo replaced"

    Write-Host "==> Restarting service" -ForegroundColor Cyan
    $started = Invoke-Remote "cd ~/$RemoteDir && pm2 restart $AppName --update-env 2>&1 | tail -6"
    foreach ($line in $started.Output) { Write-Host "    $($line.TrimEnd())" -ForegroundColor Green }

    $deadline = (Get-Date).AddSeconds(45)
    $healthy = $false
    while ((Get-Date) -lt $deadline) {
        $probe = (Invoke-Remote "curl -fsS --max-time 5 http://127.0.0.1:8050/health 2>/dev/null || true" -AllowFailure).Output -join ''
        if ($probe -match '"status":"ok"') { $healthy = $true; break }
        Start-Sleep -Seconds 2
    }
    if (-not $healthy) { throw "Service did not become healthy after DB deploy." }
    Write-Host "    health ok" -ForegroundColor Green

    $mind = (Invoke-Remote "curl -fsS --max-time 10 http://127.0.0.1:8050/ 2>/dev/null | grep -c MindPocket || true" -AllowFailure).Output -join ''
    Write-Host "    local MindPocket references: $mind" -ForegroundColor Green

    if (-not $SkipPages) {
        Write-Host "==> Queuing GitHub Pages refresh" -ForegroundColor Cyan
        $token = Get-PagesToken
        if ([string]::IsNullOrWhiteSpace($token)) {
            Write-Warning "No GitHub token found; Pages not refreshed."
        }
        else {
            $headers = @{
                Authorization = "Bearer $token"
                Accept = 'application/vnd.github+json'
                'X-GitHub-Api-Version' = '2022-11-28'
            }
            $repositoryUrl = 'https://api.github.com/repos/gideonsigilai/gideonsigilai.github.io'
            $repository = Invoke-RestMethod -Uri $repositoryUrl -Headers $headers -UserAgent 'portfolio-deploy' -TimeoutSec 30
            $body = ConvertTo-Json -InputObject @{ ref = $repository.default_branch } -Compress
            $dispatch = Invoke-WebRequest -UseBasicParsing -Method Post -Uri "$repositoryUrl/actions/workflows/pages.yml/dispatches" -Headers $headers -UserAgent 'portfolio-deploy' -ContentType 'application/json' -Body $body -TimeoutSec 30
            if ([int]$dispatch.StatusCode -ne 204) { throw "Unexpected workflow dispatch response: $($dispatch.StatusCode)" }
            Write-Host "    pages.yml queued" -ForegroundColor Green
            $headers.Clear()
        }
    }

    Write-Host ""
    Write-Host "Done. Live database updated." -ForegroundColor Green
}
finally {
    Pop-Location
    Remove-Item Env:SSHPASS -ErrorAction SilentlyContinue
}
