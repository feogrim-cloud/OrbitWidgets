param([string]$OutputDir)
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
if (-not $OutputDir) { $OutputDir = Join-Path $projectDir '.build-qa\sensor-helper' }
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
$compiler = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
if (-not (Test-Path -LiteralPath $compiler)) { throw 'Windows .NET Framework C# compiler not found.' }
$helper = Join-Path $OutputDir 'OrbitSensors.exe'
$test = Join-Path $OutputDir 'SensorHelperRegressionTests.exe'
& $compiler /nologo /target:exe /platform:x64 /optimize+ ('/out:' + $helper) (Join-Path $projectDir 'sensor-helper\Program.cs')
if ($LASTEXITCODE) { throw 'Sensor helper regression build failed.' }
& $compiler /nologo /target:exe /platform:x64 /optimize+ ('/out:' + $test) (Join-Path $projectDir 'sensor-helper\tests\RegressionTests.cs')
if ($LASTEXITCODE) { throw 'Sensor helper regression fixture build failed.' }
# The fixture uses reflection against this exact helper assembly. It never calls
# helper Main or any native hardware APIs, loads a profile, or changes Windows.
& $test $helper
if ($LASTEXITCODE) { throw 'Sensor helper regression tests failed.' }
