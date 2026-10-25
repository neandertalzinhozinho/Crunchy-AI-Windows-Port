$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$tools = Join-Path $root '.build-tools'
New-Item -ItemType Directory -Force $tools | Out-Null
$version = '10.0.26100.6584'
$packages = @{ wdk = 'microsoft.windows.wdk.x64'; sdk = 'microsoft.windows.sdk.cpp'; 'sdk-x64' = 'microsoft.windows.sdk.cpp.x64' }
foreach ($name in $packages.Keys) {
    $package = $packages[$name]
    $folder = Join-Path $tools $name
    if (Test-Path "$folder/$package.nuspec") { continue }
    $zip = Join-Path $tools "$name.zip"
    Invoke-WebRequest "https://api.nuget.org/v3-flatcontainer/$package/$version/$package.$version.nupkg" -OutFile $zip
    Expand-Archive $zip $folder -Force
}
Write-Host 'WDK and SDK ready. Install Rust stable x64-msvc if cargo is not already available.'
