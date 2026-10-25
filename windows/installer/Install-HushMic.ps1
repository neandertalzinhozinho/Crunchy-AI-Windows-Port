param([Parameter(Mandatory = $true)][string]$PayloadZip)
$ErrorActionPreference = 'Stop'
try {
    $stage = Join-Path $PSScriptRoot 'payload'
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    Expand-Archive -LiteralPath $PayloadZip -DestinationPath $stage -Force
    $installScript = Join-Path $stage 'install-local.ps1'
    if (!(Test-Path -LiteralPath $installScript)) { throw 'O pacote não contém o instalador local do HushMic.' }
    & $installScript -SourceRoot $stage
} catch {
    [Console]::Error.WriteLine(($_ | Out-String))
    exit 1
}
