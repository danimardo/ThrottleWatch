/**
 * UI-side state of the low-level sensor access ("acceso avanzado").
 *
 * The sidecar reports `available | reduced | missing | denied | error |
 * unknown` on the IPC (see contracts/ipc-protocol.md); the Rust host
 * translates that into this smaller, user-facing vocabulary (see
 * data-model.md → `low_level_access`). The design system only ever
 * receives this enum — it never sees the raw IPC state and never
 * decides whether access "is needed".
 *
 * Only `installable` and `upgradable` (an older PawnIO than the pinned
 * minimum, FR-089) render an "Instalar/Actualizar acceso avanzado"
 * action. `denied` explains that a policy or antivirus blocks it,
 * `error` offers a retry, `available`/`not_needed` render nothing
 * beyond an optional status line.
 */
export type AdvancedAccessState = 'not_needed' | 'available' | 'installable' | 'upgradable' | 'denied' | 'error';
