$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$scratch = Join-Path $root '.build-tools'
$assets = Join-Path $PSScriptRoot 'assets'
New-Item -ItemType Directory -Force $scratch, $assets | Out-Null
function Get-Verified($url, $path, $sha) {
    if ((Test-Path $path) -and (Get-FileHash $path -Algorithm SHA256).Hash -eq $sha) { return }
    Invoke-WebRequest $url -OutFile "$path.download"
    if ((Get-FileHash "$path.download" -Algorithm SHA256).Hash -ne $sha) { throw "SHA256 mismatch: $url" }
    Move-Item -LiteralPath "$path.download" -Destination $path -Force
}
$models = @{
    'dpdfnet8_48khz_hr' = '7b3afbb260a08fe9af3d16e3bda992971be1e7e951d1dee7c2d235f5c43f5631'
    'dpdfnet2_48khz_hr' = '7f0575a5cec0ba4ffd8f8bd657e06d007e4ccdd955d76faab922b9d3291dc14b'
}
foreach ($model in $models.Keys) {
    Get-Verified "https://github.com/Fovty/HushMic/releases/download/v0.11.0/$model.onnx" (Join-Path $assets "$model.onnx") $models[$model]
}
$zip = Join-Path $scratch 'onnxruntime-win-x64-1.27.0.zip'
# Digest published in the Microsoft ONNX Runtime v1.27.0 GitHub release metadata.
Get-Verified 'https://github.com/microsoft/onnxruntime/releases/download/v1.27.0/onnxruntime-win-x64-1.27.0.zip' $zip 'c5c81710938e68079ff1a192b04897faabe4b43830d48f39f27ecd4e16138bfc'
Expand-Archive -LiteralPath $zip -DestinationPath $scratch -Force
$runtime = Join-Path $scratch 'onnxruntime-win-x64-1.27.0'
Copy-Item "$runtime/lib/*.dll" $assets -Force
Copy-Item "$runtime/LICENSE" (Join-Path $assets 'ONNX-Runtime-LICENSE.txt') -Force
Copy-Item "$runtime/ThirdPartyNotices.txt" $assets -Force
Write-Host "Assets verified: $assets"
