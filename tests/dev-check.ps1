# FCR Reminder Developer Check Script (Windows PowerShell)
# Performs formatting, linting, and testing checks to ensure clean state code health.

$ErrorActionPreference = "Stop"

Write-Host "`n=== [1/3] Running Cargo Format check ===" -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Host "Format check failed! Please run 'cargo fmt --all' to fix formatting." -ForegroundColor Red
    Exit 1
}
Write-Host "Format check passed!" -ForegroundColor Green

Write-Host "`n=== [2/3] Running Cargo Clippy lints ===" -ForegroundColor Cyan
cargo clippy --all-targets --all-features -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Host "Clippy lints failed! Please fix compiler warnings/errors." -ForegroundColor Red
    Exit 1
}
Write-Host "Clippy lints passed!" -ForegroundColor Green

Write-Host "`n=== [3/3] Running Cargo Tests ===" -ForegroundColor Cyan
cargo test --all
if ($LASTEXITCODE -ne 0) {
    Write-Host "Tests failed! Please resolve failing unit tests." -ForegroundColor Red
    Exit 1
}
Write-Host "All tests passed successfully!" -ForegroundColor Green

Write-Host "`n==================================================" -ForegroundColor Green
Write-Host "  Success: Codebase is fully formatted, clean, and tested!" -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
