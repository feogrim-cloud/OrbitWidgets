param([string]$OutputDir)
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
if (-not $OutputDir) { $OutputDir = Join-Path $projectDir 'assets\screenshots' }
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
Push-Location -LiteralPath $projectDir
try {
    cargo build --release --locked --features gallery --example gallery
    if ($LASTEXITCODE) { throw 'Gallery renderer build failed.' }
    $renderer = Join-Path $projectDir 'target\release\examples\gallery.exe'
    foreach ($scene in @('hero-horizontal','theme-light','vertical','settings')) {
        $path = Join-Path $OutputDir ($scene + '.png')
        $process = Start-Process -FilePath $renderer -ArgumentList @('--scene',$scene,'--output',('"' + $path + '"')) -WindowStyle Hidden -Wait -PassThru
        if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $path)) { throw ('Gallery render failed: ' + $scene) }
        Get-Item -LiteralPath $path | Select-Object Name,Length
    }
} finally { Pop-Location }
