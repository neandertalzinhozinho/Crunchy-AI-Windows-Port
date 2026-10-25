param([string]$SourceRoot = $PSScriptRoot)
$ErrorActionPreference = 'Stop'
$source = [IO.Path]::GetFullPath($SourceRoot)
$exe = Join-Path $source 'HushMic.exe'
$assets = Join-Path $source 'assets'
if (!(Test-Path -LiteralPath $exe) -or !(Test-Path -LiteralPath $assets)) {
    throw 'Pacote do app incompleto. Extraia CrunchyAI-Windows-x64-App.zip e execute install-local.ps1 na pasta extraída.'
}
$installRoot = Join-Path $env:LOCALAPPDATA 'Programs/HushMic'
if ([IO.Path]::GetFullPath($installRoot) -eq $source) { throw 'O app já está na pasta de destino.' }
New-Item -ItemType Directory -Force -Path $installRoot | Out-Null
Copy-Item -Path (Join-Path $source '*') -Destination $installRoot -Recurse -Force
$uninstaller = Join-Path $installRoot 'uninstall-local.ps1'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\HushMic'
New-Item -Path $uninstallKey -Force | Out-Null
Set-ItemProperty $uninstallKey DisplayName 'Crunchy AI'
Set-ItemProperty $uninstallKey DisplayVersion '0.1.0'
Set-ItemProperty $uninstallKey Publisher 'Crunchy AI • Windows Port'
Set-ItemProperty $uninstallKey UninstallString "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$uninstaller`""
Set-ItemProperty $uninstallKey DisplayIcon (Join-Path $installRoot 'HushMic.exe')
Set-ItemProperty $uninstallKey NoModify 1 -Type DWord
Set-ItemProperty $uninstallKey NoRepair 1 -Type DWord
$startMenu = Join-Path $env:APPDATA 'Microsoft/Windows/Start Menu/Programs'
New-Item -ItemType Directory -Force -Path $startMenu | Out-Null
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $startMenu 'Crunchy AI.lnk'))
$shortcut.TargetPath = Join-Path $installRoot 'HushMic.exe'
$shortcut.WorkingDirectory = $installRoot
$shortcut.Save()
Write-Host "HushMic instalado em $installRoot"
Write-Host 'O VB-CABLE é instalado separadamente. No Discord/OBS, selecione CABLE Output (VB-Audio Virtual Cable).'
