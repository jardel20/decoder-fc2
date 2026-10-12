# Builds one EXE containing the GUI, TUI, icon and fonts. Run from Windows.
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
Push-Location $projectDir
try {
    cargo build --manifest-path gui/Cargo.toml --release --locked --target x86_64-pc-windows-msvc --target-dir gui/target
    if ($LASTEXITCODE -ne 0) { throw 'A compilacao falhou; nenhum portatil foi preparado.' }
    $destination = Join-Path $projectDir 'local/portable-windows'
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    Copy-Item 'gui/target/x86_64-pc-windows-msvc/release/fc2-explorer.exe' (Join-Path $destination 'fc2-explorer.exe') -Force
    Get-FileHash (Join-Path $destination 'fc2-explorer.exe') -Algorithm SHA256
    Write-Host "Portatil: $destination\fc2-explorer.exe"
    Write-Host 'Compartilhe esse EXE. Nao precisa de Rust, Leaf ou arquivos assets no computador de destino.'
} finally {
    Pop-Location
}
