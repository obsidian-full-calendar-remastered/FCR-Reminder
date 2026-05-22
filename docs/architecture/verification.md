# Verification Strategy

!!! abstract "Executable Validation"
    This page defines the preferred verification order for the daemon runtime.

## Primary Automated Check

The code-backed lifecycle smoke test lives in `src/desktop/tests/lifecycle_smoke.rs`.

Its scope is intentionally narrow:

- launch daemon
- wait for localhost endpoint to become reachable
- request stop through CLI
- verify daemon endpoint goes away
- verify the daemon child process exits

This is the primary automated regression check for daemon start/stop behavior.

## Recommended Validation Order

1. `cargo test -p desktop -- --test-threads=1`
2. `powershell -File .\src\tests\dev-check.ps1`
3. `powershell -File .\src\tests\windows-test.ps1 -StartDaemon -SeedReminder`

!!! note "Validation Philosophy"
    Use the narrowest executable check that can falsify the behavior you changed before widening into broader scripted or manual verification.

Compact index: [Architecture Docs](index.md) · [Control API and Lifecycle](control_api.md) · [Windows Setup](../user/windows_setup.md)