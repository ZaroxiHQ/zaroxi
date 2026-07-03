#!/usr/bin/env pwsh
#
# run-ci-windows.ps1 - run the important CI gates locally on Windows, mirroring
# tooling/scripts/run-ci-local.sh (which is bash / Unix-focused).
#
# Use this on a Windows machine (or a Windows CI runner) to reproduce the
# pipeline before pushing. It runs every gate (does NOT stop on first failure),
# prints a summary, and exits non-zero if any gate failed.
#
# Requirements on Windows:
#   - Rust (MSVC toolchain), cargo
#   - Git Bash (for the two bash scripts below); ships with Git for Windows and
#     is present on GitHub 'windows-latest' runners
#   - Python 3 (python3 or python on PATH)
#   - cargo-audit and cargo-deny (cargo install cargo-audit cargo-deny --locked)
#
# Usage:
#   pwsh -File tooling/scripts/run-ci-windows.ps1

$ErrorActionPreference = "Continue"

# Resolve repo root from this script's location (tooling/scripts/..).
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $RepoRoot

# Prefer python3, fall back to python.
$Py = if (Get-Command python3 -ErrorAction SilentlyContinue) { "python3" } else { "python" }

$script:Passed = @()
$script:Failed = @()

function Invoke-Gate {
    param([string]$Name, [scriptblock]$Body)
    Write-Host ""
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host ">> $Name" -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan
    & $Body
    if ($LASTEXITCODE -eq 0) {
        $script:Passed += $Name
    } else {
        $script:Failed += $Name
        Write-Host "FAILED: $Name (exit $LASTEXITCODE)" -ForegroundColor Red
    }
}

Invoke-Gate "fmt --check"                 { cargo fmt --all -- --check }
Invoke-Gate "check --workspace"           { cargo check --workspace --all-targets }
Invoke-Gate "build --workspace"           { cargo build --workspace }
Invoke-Gate "clippy (default)"            { cargo clippy --workspace --all-targets -- -D warnings }
Invoke-Gate "clippy (all-features)"       { cargo clippy --workspace --all-targets --all-features -- -D warnings }

# Build the Tree-sitter grammars for windows-x86_64 before the syntax tests
# (uses Git Bash + the MSVC C toolchain via the cc crate).
Invoke-Gate "prepare tree-sitter"         { bash tooling/scripts/prepare-treesitter.sh }

Invoke-Gate "test --workspace"            { cargo test --workspace }
Invoke-Gate "test explorer_integration"   { cargo test -p zaroxi-interface-desktop --test explorer_integration }
Invoke-Gate "test resolve_dirty_close"    { cargo test -p zaroxi-application-workspace --test resolve_dirty_close }
Invoke-Gate "test highlight_spans"        { cargo test -p zaroxi-core-platform-syntax --test highlight_spans }

Invoke-Gate "check_circular_deps.py"      { & $Py .github/scripts/check_circular_deps.py }
Invoke-Gate "check_crate_naming.py"       { & $Py .github/scripts/check_crate_naming.py }

Invoke-Gate "cargo audit"                 { cargo audit }
Invoke-Gate "cargo deny check"            { cargo deny check }

# Bash architecture checker (run via Git Bash).
Invoke-Gate "architecture_check.sh"       { bash scripts/architecture_check.sh }

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "Windows CI-local summary"
Write-Host "============================================================" -ForegroundColor Cyan
foreach ($p in $script:Passed) { Write-Host "  PASS  $p" -ForegroundColor Green }
foreach ($f in $script:Failed) { Write-Host "  FAIL  $f" -ForegroundColor Red }
Write-Host "------------------------------------------------------------"
Write-Host ("  {0} passed, {1} failed" -f $script:Passed.Count, $script:Failed.Count)

exit $script:Failed.Count
