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
 * beyond an optional status line. `capped_by_vendor` (2026-09-28):
 * installed and working, but this processor's vendor has no
 * limitation-reason registry yet, so the best reachable coverage is
 * level B — a fact, not a fault, so it renders like `available` (a
 * status line, no retry action) with its own wording.
 */
export type AdvancedAccessState =
  | 'not_needed'
  | 'available'
  | 'installable'
  | 'upgradable'
  | 'denied'
  | 'error'
  | 'capped_by_vendor';
