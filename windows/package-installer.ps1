$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
& (Join-Path $PSScriptRoot 'build-app.ps1') -SkipAssets
if ($LASTEXITCODE) { throw 'A compilação do HushMic falhou.' }
$app = Join-Path $root 'dist/HushMic-Windows-x64'
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'install-local.ps1') -Destination (Join-Path $app 'install-local.ps1') -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'uninstall-local.ps1') -Destination (Join-Path $app 'uninstall-local.ps1') -Force
$zip = Join-Path $root 'dist/CrunchyAI-Windows-x64-App.zip'
$packageFiles = [ordered]@{
    'HushMic.exe' = 'HushMic.exe'
    'assets/crunchy-ai-icon.png' = 'assets/crunchy-ai-icon.png'
    'assets/crunchy-ai.ico' = 'assets/crunchy-ai.ico'
    'install-local.ps1' = 'install-local.ps1'
    'uninstall-local.ps1' = 'uninstall-local.ps1'
    'LICENSE-APACHE' = 'LICENSE-APACHE'
    'LICENSE-MIT' = 'LICENSE-MIT'
    'README.md' = 'README.md'
    'THIRD_PARTY_NOTICES.md' = 'THIRD_PARTY_NOTICES.md'
    'assets/dpdfnet2_48khz_hr.onnx' = 'assets/dpdfnet2_48khz_hr.onnx'
    'assets/dpdfnet8_48khz_hr.onnx' = 'assets/dpdfnet8_48khz_hr.onnx'
    'assets/ONNX-Runtime-LICENSE.txt' = 'assets/ONNX-Runtime-LICENSE.txt'
    'assets/onnxruntime_providers_shared.dll' = 'assets/onnxruntime_providers_shared.dll'
    'assets/onnxruntime.dll' = 'assets/onnxruntime.dll'
    'assets/ThirdPartyNotices.txt' = 'assets/ThirdPartyNotices.txt'
}
foreach ($entry in $packageFiles.GetEnumerator()) {
    if (!(Test-Path -LiteralPath (Join-Path $app $entry.Value) -PathType Leaf)) {
        throw "Arquivo obrigatório ausente do pacote: $($entry.Value)"
    }
}
if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
$archiveFile = [IO.File]::Open($zip, [IO.FileMode]::CreateNew)
try {
    $archive = [IO.Compression.ZipArchive]::new($archiveFile, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($entry in $packageFiles.GetEnumerator()) {
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                $archive,
                (Join-Path $app $entry.Value),
                $entry.Key,
                [IO.Compression.CompressionLevel]::Optimal
            ) | Out-Null
        }
    } finally {
        $archive.Dispose()
    }
} finally {
    $archiveFile.Dispose()
}
$archive = [IO.Compression.ZipFile]::OpenRead($zip)
try {
    $entries = @($archive.Entries | ForEach-Object { $_.FullName })
    foreach ($required in @(
        'HushMic.exe',
        'assets/crunchy-ai-icon.png',
        'assets/crunchy-ai.ico',
        'install-local.ps1',
        'uninstall-local.ps1',
        'assets/dpdfnet2_48khz_hr.onnx',
        'assets/dpdfnet8_48khz_hr.onnx',
        'assets/onnxruntime.dll'
    )) {
        if ($entries -notcontains $required) { throw "Arquivo obrigatório ausente do pacote: $required" }
    }
    $forbidden = @($entries | Where-Object {
        $_ -match '(?i)(^|/)vbcable(/|$)|\.(sys|cat|inf|cer)$|VBCABLE_Setup'
    })
    if ($forbidden.Count) {
        throw "O ZIP app-only contém arquivo de driver proibido: $($forbidden -join ', ')"
    }
    if ($entries.Count -ne $packageFiles.Count) {
        throw "O ZIP deve conter exatamente $($packageFiles.Count) arquivos app-only; encontrados: $($entries.Count)."
    }
} finally {
    $archive.Dispose()
}
$frameworks = @(
    (Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'),
    (Join-Path $env:WINDIR 'Microsoft.NET/Framework/v4.0.30319/csc.exe')
)
$compiler = $frameworks | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (!$compiler) { throw 'O compilador C# do .NET Framework não está disponível para criar o instalador único.' }
$setup = Join-Path $root 'dist/CrunchyAI-Setup-x64.exe'
$compilerArgs = @(
    '/nologo',
    '/target:winexe',
    '/platform:x64',
    "/out:$setup",
    "/win32icon:$(Join-Path $PSScriptRoot 'assets/crunchy-ai.ico')",
    "/win32manifest:$(Join-Path $PSScriptRoot 'installer/app.manifest')",
    '/reference:System.IO.Compression.dll',
    '/reference:System.Windows.Forms.dll',
    "/resource:$zip,HushMicPayload.zip",
    "/resource:$(Join-Path $PSScriptRoot 'installer/Install-HushMic.ps1'),Install-HushMic.ps1",
    (Join-Path $PSScriptRoot 'installer/Bootstrap.cs')
)
& $compiler @compilerArgs
if ($LASTEXITCODE) { throw 'A compilação do instalador HushMic falhou.' }
if (!(Test-Path -LiteralPath $setup) -or (Get-Item -LiteralPath $setup).Length -lt 1000000) {
    throw 'O executável de instalação único está ausente ou incompleto.'
}
$setupBytes = [IO.File]::ReadAllBytes($setup)
$peOffset = [BitConverter]::ToInt32($setupBytes, 0x3c)
$machine = [BitConverter]::ToUInt16($setupBytes, $peOffset + 4)
if ($machine -ne 0x8664) { throw ('O instalador não é x64 (COFF machine 0x{0:X4}).' -f $machine) }
Write-Host "Instalador único por usuário: $setup"
Write-Host "Pacote portátil app-only: $zip"
Write-Host 'O VB-CABLE não está incluído; instale-o separadamente.'
