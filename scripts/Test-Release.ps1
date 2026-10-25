param(
    [string]$ReleaseDir = 'release',
    [string]$StageDir = 'dist',
    [string]$QAInstaller
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$ReleaseDir = (Resolve-Path -LiteralPath $ReleaseDir).Path
$StageDir = (Resolve-Path -LiteralPath $StageDir).Path
$expectedAssets = @('OrbitWidgets-Setup-x64.exe','OrbitWidgets-Windows-x64.zip','OrbitWidgets-source.zip')
$manifest = Get-Content -LiteralPath (Join-Path $ReleaseDir 'SHA256SUMS.txt')
if ($manifest.Count -ne $expectedAssets.Count) { throw 'Checksum manifest must list exactly three assets.' }
foreach ($asset in $expectedAssets) {
    $line = @($manifest | Where-Object { $_ -match ('^[a-fA-F0-9]{64}  ' + [regex]::Escape($asset) + '$') })
    if ($line.Count -ne 1) { throw ('Missing or duplicate checksum: ' + $asset) }
    if ((Get-FileHash -LiteralPath (Join-Path $ReleaseDir $asset)).Hash -ne $line[0].Substring(0,64)) { throw ('Checksum mismatch: ' + $asset) }
}
$executable = Join-Path $StageDir 'OrbitWidgets.exe'
$bytes = [IO.File]::ReadAllBytes($executable)
$peOffset = [BitConverter]::ToInt32($bytes,0x3c)
if ($bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a -or [BitConverter]::ToUInt32($bytes,$peOffset) -ne 0x4550 -or [BitConverter]::ToUInt16($bytes,$peOffset+4) -ne 0x8664) { throw 'Application is not a Windows x64 PE executable.' }
# Read actual PE imports, not arbitrary strings embedded in the executable.
$optionalHeader = $peOffset + 24
if ([BitConverter]::ToUInt16($bytes,$optionalHeader) -ne 0x20b) { throw 'Expected a PE32+ optional header.' }
$sectionCount = [BitConverter]::ToUInt16($bytes,$peOffset+6)
$sectionTable = $optionalHeader + [BitConverter]::ToUInt16($bytes,$peOffset+20)
function Convert-ImportRva([uint32]$rva) {
    for ($i=0; $i -lt $sectionCount; $i++) {
        $section = $sectionTable + 40*$i
        $address = [BitConverter]::ToUInt32($bytes,$section+12)
        $length = [Math]::Max([BitConverter]::ToUInt32($bytes,$section+8),[BitConverter]::ToUInt32($bytes,$section+16))
        if ($rva -ge $address -and $rva -lt $address+$length) { return [int]([BitConverter]::ToUInt32($bytes,$section+20)+$rva-$address) }
    }
    throw 'Invalid PE import address.'
}
$importRva = [BitConverter]::ToUInt32($bytes,$optionalHeader+120)
if ($importRva) {
    $descriptor = Convert-ImportRva $importRva
    while ($true) {
        $nameRva = [BitConverter]::ToUInt32($bytes,$descriptor+12)
        if (-not $nameRva) { break }
        $offset = Convert-ImportRva $nameRva
        $end = $offset
        while ($end -lt $bytes.Length -and $bytes[$end] -ne 0) { $end++ }
        $importName = [Text.Encoding]::ASCII.GetString($bytes,$offset,$end-$offset)
        if ($importName -match '^(VCRUNTIME|MSVCP|libgcc|libstdc\+\+|libwinpthread)') { throw ('Portable application requires an external compiler runtime: ' + $importName) }
        $descriptor += 20
    }
}
$stagedHash = (Get-FileHash -LiteralPath $executable).Hash
$portable = [IO.Compression.ZipFile]::OpenRead((Join-Path $ReleaseDir 'OrbitWidgets-Windows-x64.zip'))
try {
    foreach ($entry in $portable.Entries) {
        if ($entry.FullName -match '(^/|\\|(^|/)\.\.(/|$))') { throw 'Unsafe portable archive path.' }
    }
    foreach ($name in @('OrbitWidgets.exe','sensors/OrbitSensors.exe','sensors/LibreHardwareMonitorLib.dll','optional/PawnIO-Setup.exe','SENSOR-SOURCES.txt','LICENSE')) {
        if (-not $portable.GetEntry('OrbitWidgets/' + $name)) { throw ('Incomplete portable archive: ' + $name) }
    }
    $stream = $portable.GetEntry('OrbitWidgets/OrbitWidgets.exe').Open()
    try {
        $hasher = [Security.Cryptography.SHA256]::Create()
        try { $archiveHash = [BitConverter]::ToString($hasher.ComputeHash($stream)).Replace('-','') } finally { $hasher.Dispose() }
    } finally { $stream.Dispose() }
    if ($archiveHash -ne $stagedHash) { throw 'Portable executable differs from the staged build.' }
} finally { $portable.Dispose() }
$source = [IO.Compression.ZipFile]::OpenRead((Join-Path $ReleaseDir 'OrbitWidgets-source.zip'))
try {
    foreach ($entry in $source.Entries) {
        if ($entry.FullName -match '(^/|\\|(^|/)\.\.(/|$))|(^|/)(target|dist|\.git|\.source|work)/|settings\.json|cargo-metadata\.json|\.log$') { throw ('Unexpected private or generated source entry: ' + $entry.FullName) }
    }
    foreach ($name in @('Cargo.toml','Cargo.lock','README.md','README.ru.md','docs/BUILD.md','.github/workflows/build.yml','scripts/package.py','src/ui.rs','sensor-helper/Program.cs')) {
        if (-not $source.GetEntry('OrbitWidgets-source/' + $name)) { throw ('Incomplete source archive: ' + $name) }
    }
} finally { $source.Dispose() }
if ($QAInstaller) {
    $QAInstaller = (Resolve-Path -LiteralPath $QAInstaller).Path
    if ([IO.Path]::GetFileName($QAInstaller) -ne 'OrbitWidgets-Setup-QA.exe') { throw 'Only the file-copy-only QA installer can run in this check.' }
    $qaDirectory = Join-Path (Split-Path $QAInstaller -Parent) ('installed-' + [Guid]::NewGuid().ToString('N'))
    $process = Start-Process -FilePath $QAInstaller -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/DIR="' + $qaDirectory + '"')) -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw ('QA installer failed: ' + $process.ExitCode) }
    if ((Get-FileHash -LiteralPath (Join-Path $qaDirectory 'OrbitWidgets.exe')).Hash -ne $stagedHash) { throw 'QA installer copied a different application build.' }
}
Write-Output 'Release checks passed: x64 executable, complete archives, checksums and optional QA installer.'
