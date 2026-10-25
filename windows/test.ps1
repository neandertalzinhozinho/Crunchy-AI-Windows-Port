$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$localCargo = Join-Path $root '.build-tools/cargo/bin/cargo.exe'
if (Test-Path $localCargo) {
    $env:CARGO_HOME = Join-Path $root '.build-tools/cargo'
    $env:RUSTUP_HOME = Join-Path $root '.build-tools/rustup'
    $cargo = $localCargo
} else { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
& "$PSScriptRoot/setup-assets.ps1"
New-Item -ItemType Directory -Force "$root/assets/lib", "$root/assets/models" | Out-Null
Copy-Item "$PSScriptRoot/assets/*.dll" "$root/assets/lib" -Force
Copy-Item "$PSScriptRoot/assets/*.onnx" "$root/assets/models" -Force
$env:HUSHMIC_ASSERT_ASSETS = '1'
& $cargo test --manifest-path "$root/Cargo.toml" --locked -p hushmic-denoiser --release --lib --test api --test latency --test parity
if ($LASTEXITCODE) { throw 'Denoiser regression tests failed' }
& $cargo test --manifest-path "$PSScriptRoot/app/Cargo.toml" --locked
if ($LASTEXITCODE) { throw 'Windows application tests failed' }
& $cargo clippy --manifest-path "$PSScriptRoot/app/Cargo.toml" --locked -- -D warnings
if ($LASTEXITCODE) { throw 'Windows application lint failed' }
$uninstallGuardOutput = (& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'uninstall-local.ps1') 2>&1 | Out-String)
$uninstallGuardExitCode = $LASTEXITCODE
if ($uninstallGuardExitCode -eq 0 -or $uninstallGuardOutput -notmatch 'Recusa remover arquivos fora da pasta HushMic instalada') {
    throw 'Uninstaller path guard did not reject a non-installation path'
}
& "$PSScriptRoot/build-app.ps1" -SkipAssets
& "$root/dist/HushMic-Windows-x64/HushMic.exe" --self-test
if ($LASTEXITCODE) { throw 'Packaged application smoke test failed' }
& "$PSScriptRoot/package-installer.ps1"
if ($LASTEXITCODE) { throw 'Single-file installer build failed' }
& "$root/dist/CrunchyAI-Setup-x64.exe" --verify
if ($LASTEXITCODE) { throw 'Installer payload validation failed' }
