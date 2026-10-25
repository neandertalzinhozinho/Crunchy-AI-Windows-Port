$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$installRoot = [IO.Path]::GetFullPath($PSScriptRoot)
$expectedRoot = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs/HushMic'))
if (![StringComparer]::OrdinalIgnoreCase.Equals($installRoot, $expectedRoot)) {
    throw "Recusa remover arquivos fora da pasta HushMic instalada: $installRoot"
}
$choice = [Windows.Forms.MessageBox]::Show('Isso removerá o aplicativo Crunchy AI e seus arquivos. O driver VB-CABLE, instalado separadamente, não será alterado. Continuar?', 'Desinstalar Crunchy AI', 'YesNo', 'Warning')
if ($choice -ne [Windows.Forms.DialogResult]::Yes) { exit 0 }
Remove-Item -LiteralPath (Join-Path $env:APPDATA 'Microsoft/Windows/Start Menu/Programs/Crunchy AI.lnk') -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\HushMic' -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $installRoot -Recurse -Force
