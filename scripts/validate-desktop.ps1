# Run on Windows with Rust stable, MSVC, Windows SDK, rustfmt and Clippy.
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
$logDir = Join-Path $projectDir ("local/desktop-validation/" + (Get-Date -Format 'yyyyMMdd-HHmmss') + "-$PID")
New-Item -ItemType Directory -Path $logDir -Force | Out-Null
$logFile = Join-Path $logDir 'build.log'
$desktopTarget = 'x86_64-pc-windows-msvc'
function Invoke-Step {
    param([string]$Label, [string]$Command, [string[]]$CommandArgs)
    Get-Command $Command -ErrorAction Stop | Out-Null
    "`nEtapa: $Label" | Tee-Object -FilePath $logFile -Append
    # Continue allows PowerShell 5.1 to capture native stderr diagnostics.
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $Command @CommandArgs 2>&1 | Tee-Object -FilePath $logFile -Append
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($code -ne 0) { throw "Falha em $Label (codigo $code). Log: $logFile" }
}
Push-Location $projectDir
try {
    "Validacao desktop FC2 Explorer. Log: $logFile" | Tee-Object -FilePath $logFile -Append
    Invoke-Step 'Compilador' 'rustc' @('--version')
    Invoke-Step 'Cargo' 'cargo' @('--version')
    Invoke-Step 'Dependencias do nucleo' 'cargo' @('fetch', '--locked', '--target', $desktopTarget)
    $guiFetch = @('fetch', '--manifest-path', 'gui/Cargo.toml', '--target', $desktopTarget)
    if (Test-Path 'gui/Cargo.lock') { $guiFetch += '--locked' }
    Invoke-Step 'Dependencias GUI e lockfile' 'cargo' $guiFetch
    Invoke-Step 'Formatacao do nucleo' 'cargo' @('fmt', '--all', '--', '--check')
    Invoke-Step 'Formatacao GUI' 'cargo' @('fmt', '--manifest-path', 'gui/Cargo.toml', '--all', '--', '--check')
    Invoke-Step 'Verificacao do nucleo' 'cargo' @('check', '--workspace', '--locked', '--target', $desktopTarget)
    Invoke-Step 'Testes do nucleo' 'cargo' @('test', '--workspace', '--locked', '--target', $desktopTarget)
    Invoke-Step 'Clippy do nucleo' 'cargo' @('clippy', '--workspace', '--all-targets', '--locked', '--target', $desktopTarget, '--', '-D', 'warnings')
    Invoke-Step 'Verificacao GUI' 'cargo' @('check', '--manifest-path', 'gui/Cargo.toml', '--locked', '--target', $desktopTarget)
    Invoke-Step 'Testes GUI' 'cargo' @('test', '--manifest-path', 'gui/Cargo.toml', '--locked', '--target', $desktopTarget)
    Invoke-Step 'Clippy GUI' 'cargo' @('clippy', '--manifest-path', 'gui/Cargo.toml', '--all-targets', '--locked', '--target', $desktopTarget, '--', '-D', 'warnings')
    Invoke-Step 'Executavel CLI/TUI' 'cargo' @('build', '--release', '--locked', '--target', $desktopTarget, '--target-dir', 'target')
    Invoke-Step 'Executavel GUI' 'cargo' @('build', '--manifest-path', 'gui/Cargo.toml', '--release', '--locked', '--target', $desktopTarget, '--target-dir', 'gui/target')
    "GUI: gui/target/$desktopTarget/release/fc2-explorer.exe" | Tee-Object -FilePath $logFile -Append
    'Verificacoes concluidas. Execute os testes manuais de docs/desktop-validation.md.' | Tee-Object -FilePath $logFile -Append
} finally {
    Pop-Location
}
