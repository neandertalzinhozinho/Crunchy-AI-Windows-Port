param([switch]$SkipAssets)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$localCargo = Join-Path $root '.build-tools/cargo/bin/cargo.exe'
if (Test-Path $localCargo) {
    $env:CARGO_HOME = Join-Path $root '.build-tools/cargo'
    $env:RUSTUP_HOME = Join-Path $root '.build-tools/rustup'
    $cargo = $localCargo
} else { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
if (!$SkipAssets) { & "$PSScriptRoot/setup-assets.ps1" }
& $cargo build --manifest-path "$PSScriptRoot/app/Cargo.toml" --release --locked
if ($LASTEXITCODE) { throw 'Application build failed' }
$dist = Join-Path $root 'dist/HushMic-Windows-x64'
New-Item -ItemType Directory -Force "$dist/assets" | Out-Null
Copy-Item "$PSScriptRoot/app/target/release/hushmic.exe" "$dist/HushMic.exe" -Force
Copy-Item "$PSScriptRoot/assets/*" "$dist/assets" -Force
# Embed the Crunchy AI logo as the Windows executable icon (Explorer/taskbar).
$icon = Join-Path $PSScriptRoot 'assets/crunchy-ai.ico'
if (!(Test-Path -LiteralPath $icon)) { throw 'Ícone Crunchy AI ausente em windows/assets/crunchy-ai.ico.' }
$iconCompiler = Join-Path $PSScriptRoot 'app/target/release/icon-compiler.exe'
if (Test-Path -LiteralPath $iconCompiler) { Remove-Item -LiteralPath $iconCompiler -Force }
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class CrunchyIconCompiler {
    [DllImport("kernel32", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr BeginUpdateResource(string file, bool deleteExisting);
    [DllImport("kernel32", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern bool UpdateResource(IntPtr update, IntPtr type, IntPtr name, ushort language, byte[] data, uint size);
    [DllImport("kernel32", SetLastError=true)]
    public static extern bool EndUpdateResource(IntPtr update, bool discard);
}
'@
Copy-Item -LiteralPath "$PSScriptRoot/app/target/release/hushmic.exe" -Destination $iconCompiler -Force
$update = [CrunchyIconCompiler]::BeginUpdateResource($iconCompiler, $false)
if ($update -eq [IntPtr]::Zero) { throw "Não abriu o executável para incorporar ícone: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())" }
try {
    $iconBytes = [IO.File]::ReadAllBytes($icon)
    $count = [BitConverter]::ToUInt16($iconBytes, 4)
    if ($count -lt 1) { throw 'O arquivo ICO não contém imagens.' }
    $group = [IO.MemoryStream]::new()
    $writer = [IO.BinaryWriter]::new($group)
    $writer.Write([UInt16]0); $writer.Write([UInt16]1); $writer.Write($count)
    for ($i = 0; $i -lt $count; $i++) {
        $entryOffset = 6 + 16 * $i
        $imageLength = [BitConverter]::ToUInt32($iconBytes, $entryOffset + 8)
        $imageOffset = [BitConverter]::ToUInt32($iconBytes, $entryOffset + 12)
        $image = [byte[]]::new($imageLength)
        [Array]::Copy($iconBytes, $imageOffset, $image, 0, $imageLength)
        $iconId = [UInt16]($i + 1)
        $iconType = [IntPtr]3
        $iconName = [IntPtr]$iconId
        if (![CrunchyIconCompiler]::UpdateResource($update, $iconType, $iconName, 0, $image, [uint32]$image.Length)) {
            throw "Falha ao incorporar imagem do ícone: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
        }
        $writer.Write($iconBytes, $entryOffset, 12)
        $writer.Write($iconId)
    }
    $writer.Flush()
    $groupBytes = $group.ToArray()
    $groupType = [IntPtr]14 # RT_GROUP_ICON, required for Explorer/taskbar lookup
    $groupName = [IntPtr]1
    if (![CrunchyIconCompiler]::UpdateResource($update, $groupType, $groupName, 0, $groupBytes, [uint32]$groupBytes.Length)) {
        throw "Falha ao incorporar grupo de ícones: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    }
    $writer.Dispose(); $group.Dispose()
    if (![CrunchyIconCompiler]::EndUpdateResource($update, $false)) {
        throw "Falha ao gravar ícone no executável: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    }
    $update = [IntPtr]::Zero
} finally {
    if ($update -ne [IntPtr]::Zero) { [CrunchyIconCompiler]::EndUpdateResource($update, $true) | Out-Null }
}
Copy-Item -LiteralPath $iconCompiler -Destination (Join-Path $dist 'HushMic.exe') -Force
Remove-Item -LiteralPath $iconCompiler -Force
Copy-Item "$root/LICENSE-MIT", "$root/LICENSE-APACHE", "$PSScriptRoot/README.md", "$PSScriptRoot/THIRD_PARTY_NOTICES.md" $dist -Force
Write-Host "Application: $dist/HushMic.exe"
