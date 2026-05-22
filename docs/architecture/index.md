# Architecture Docs

!!! abstract "Philosophy"
    These docs are written to be comprehensive and navigable at the same time. We prefer compact, high-signal sections over long mixed-purpose pages, and we separate routing pages from content pages so readers can choose the exact level they need.

!!! info "Two Audiences, One Contract"
    User Docs describe workflows and practical operation. Architecture Docs define implementation boundaries, invariants, control surfaces, and extension contracts. Both tracks must remain consistent.

!!! warning "Source-of-Truth Rule"
    Architecture docs define the implementation authority for the documented runtime. If behavior diverges from this section, treat it as a defect and update either code or docs deliberately.

## Decision Matrix

| Question | Start here | Related deep dive |
|---|---|---|
| What is the daemon responsible for? | [Runtime Overview](architecture.md) | [Implementation Blueprint](blueprint.md) |
| How does a host talk to the daemon safely? | [Control API and Lifecycle](control_api.md) | [Developer Integration Guide](../developer/integration_guide.md) |
| Where does Windows-specific behavior live? | [Windows Runtime](windows_runtime.md) | [Implementation Blueprint](blueprint.md#4-windows-specific-ownership) |
| How is lifecycle behavior validated? | [Verification Strategy](verification.md) | [Windows Setup](../user/windows_setup.md) |
| Where should I change code for a given behavior? | [Implementation Blueprint](blueprint.md) | [Runtime Overview](architecture.md) |

## Scope

This section is concept-first and implementation-bound. It documents ownership, data movement, invariants, extension points, and verification policy.

## Implementation Anchors

Daemon control plane: `src/desktop/src/main.rs`  
Shared storage and logging: `src/reminder_core/src/storage.rs`, `src/reminder_core/src/logger.rs`  
CLI forwarding: `src/desktop/src/cli/mod.rs`  
Windows integrations: `src/desktop/src/platform/windows/*`


---

[Runtime Overview](architecture.md) · [Control API and Lifecycle](control_api.md) · [Windows Runtime](windows_runtime.md) · [Verification Strategy](verification.md) · [Blueprint](blueprint.md)