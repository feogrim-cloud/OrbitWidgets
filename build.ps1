param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$localTools = Join-Path (Split-Path $PSScriptRoot -Parent) 'toolchain'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $localCargo = Join-Path $localTools 'cargo'
    if (-not (Test-Path -LiteralPath (Join-Path $localCargo 'bin\cargo.exe'))) {
        throw 'Install Rust (https://rustup.rs/) and a MinGW toolchain, or use MSVC Rust with Visual Studio C++ Build Tools.'
    }
    $env:CARGO_HOME = $localCargo
    $env:RUSTUP_HOME = Join-Path $localTools 'rustup'
    $env:PATH = (Join-Path $localCargo 'bin') + ';' + $env:PATH
}
$mingwCandidates = @((Join-Path $localTools 'w64devkit\bin'),(Join-Path $localTools 'w64devkit\w64devkit\bin'))
foreach ($candidate in $mingwCandidates) {
    if (Test-Path -LiteralPath (Join-Path $candidate 'gcc.exe')) { $env:PATH = $candidate + ';' + $env:PATH; break }
}
$env:CARGO_BUILD_JOBS = '8'
if (-not $SkipTests) { cargo test --release --locked; if ($LASTEXITCODE) { throw 'Rust tests failed.' } }
cargo build --release --locked
if ($LASTEXITCODE) { throw 'Rust build failed.' }
$cscPath = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
& $cscPath /nologo /target:exe /platform:x64 /optimize+ '/out:build-assets\sensors\OrbitSensors.exe' 'sensor-helper\Program.cs'
if ($LASTEXITCODE) { throw 'Sensor helper build failed.' }
New-Item -ItemType Directory -Path 'dist\sensors','dist\optional' -Force | Out-Null
Copy-Item -LiteralPath 'target\release\orbit-widgets.exe' -Destination 'dist\OrbitWidgets.exe'
Get-ChildItem -LiteralPath 'build-assets\sensors' -File | Copy-Item -Destination 'dist\sensors'
Copy-Item -LiteralPath 'build-assets\sensors\licenses' -Destination 'dist\sensors' -Recurse -Force
Get-ChildItem -LiteralPath 'build-assets\optional' -File | Copy-Item -Destination 'dist\optional'
Copy-Item -LiteralPath 'build-assets\SENSOR-SOURCES.txt','LICENSE','README-RU.txt' -Destination 'dist'
if (Test-Path -LiteralPath 'build-assets\THIRD-PARTY-RUST.txt') { Copy-Item -LiteralPath 'build-assets\THIRD-PARTY-RUST.txt' -Destination 'dist' }
if (Test-Path -LiteralPath 'build-assets\licenses') { Copy-Item -LiteralPath 'build-assets\licenses' -Destination 'dist' -Recurse -Force }
if (Test-Path -LiteralPath 'build-assets\orbit.ico') { Copy-Item -LiteralPath 'build-assets\orbit.ico' -Destination 'dist' }
Write-Output (Join-Path $PSScriptRoot 'dist\OrbitWidgets.exe')
