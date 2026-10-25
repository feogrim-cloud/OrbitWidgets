param(
    [string]$SourceDir,
    [string]$OutputDir,
    [string]$Compiler,
    [string]$AppVersion = '1.1.1',
    [switch]$QA
)
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
$workspaceDir = Split-Path -Parent (Split-Path -Parent $projectDir)
if (-not $SourceDir) { $SourceDir = Join-Path $projectDir 'dist' }
if (-not $OutputDir) {
    $OutputDir = if ($QA) { Join-Path (Split-Path -Parent $projectDir) 'installer-qa-build' } else { Join-Path $workspaceDir 'outputs' }
}
if (-not $Compiler) { $Compiler = Join-Path (Split-Path -Parent $projectDir) 'toolchain\inno\ISCC.exe' }
$SourceDir = (Resolve-Path -LiteralPath $SourceDir).Path
$assetsDir = Join-Path $projectDir 'build-assets'
$driver = Join-Path $assetsDir 'optional\PawnIO-Setup.exe'
if (-not (Test-Path -LiteralPath (Join-Path $SourceDir 'OrbitWidgets.exe'))) { throw 'Stage the app build in dist before compiling the installer.' }
if (-not (Test-Path -LiteralPath $Compiler)) { throw 'Inno Setup 7.1 compiler not found. Pass -Compiler with its ISCC.exe path.' }
$expectedDriverHash = '1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032'
if ((Get-FileHash -LiteralPath $driver -Algorithm SHA256).Hash -ne $expectedDriverHash) { throw 'PawnIO official release hash mismatch.' }
$signature = Get-AuthenticodeSignature -LiteralPath $driver
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=namazso\.eu') { throw 'PawnIO official Authenticode signature is not valid.' }
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
$arguments = @('/Qp',('/DSourceDir=' + $SourceDir),('/DBuildAssetsDir=' + $assetsDir),('/DOutputDir=' + $OutputDir),('/DAppVersion=' + $AppVersion))
if ($QA) { $arguments += '/DOrbitSetupQA=1' }
$arguments += (Join-Path $PSScriptRoot 'OrbitWidgets.iss')
& $Compiler @arguments
if ($LASTEXITCODE -ne 0) { throw ('Inno Setup compiler failed with exit code ' + $LASTEXITCODE) }
$filename = if ($QA) { 'OrbitWidgets-Setup-QA.exe' } else { 'OrbitWidgets-Setup-x64.exe' }
Get-Item -LiteralPath (Join-Path $OutputDir $filename) | Select-Object FullName,Length
