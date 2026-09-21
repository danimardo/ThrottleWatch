//! Ciclo de vida del cierre de ventana (`contracts/application-commands.md`, «Ventana y ciclo de
//! vida»): Rust decide qué hacer con un `CloseRequested` — nativo (Alt+F4, gestor de ventanas) o
//! desde el botón personalizado de `TitleBar`, que emiten el mismo evento del motor de ventanas —
//! sin depender de lo que crea el frontend, que puede no estar respondiendo. Dos decisiones
//! comparten este módulo porque comparten el mismo punto de entrada (`CloseRequested`) y no deben
//! pisarse: si hay una operación en curso, eso siempre gana (T090); si no, y no hay una elección
//! `lifecycle.close_action` guardada todavía, hace falta preguntar (T182, FR-046). Las funciones
//! puras (`reason_for`, `confirm_close_outcome`, `intent_for`) deciden a partir de valores ya
//! leídos, así se prueban sin un `AppHandle`.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::updates::Blocker;
use tauri::{AppHandle, Manager};

pub const REASON_GUIDED: &str = "guided";
pub const REASON_INSTALL: &str = "install";
pub const REASON_DOWNLOAD: &str = "download";
pub const REASON_EXPORT: &str = "export";

/// El evento que pide la decisión del primer cierre (`resolve_first_close` la responde).
pub const DECISION_REQUIRED_EVENT: &str = "lifecycle:close-decision-required";

/// Prioridad cuando varias cosas ocurren a la vez: una prueba guiada en curso es una cuestión de
/// seguridad y gana siempre; una instalación no puede interrumpirse nunca; una descarga no ha
/// escrito nada todavía pero conviene confirmarla igual; una exportación o importación dejarían
/// un archivo a medias o una transacción abierta.
pub fn reason_for(
    guided_running: bool,
    update_state: &str,
    busy: Option<Blocker>,
) -> Option<&'static str> {
    if guided_running {
        return Some(REASON_GUIDED);
    }
    match update_state {
        "installing" => return Some(REASON_INSTALL),
        "downloading" => return Some(REASON_DOWNLOAD),
        _ => {}
    }
    if matches!(busy, Some(Blocker::Export) | Some(Blocker::Import)) {
        return Some(REASON_EXPORT);
    }
    None
}

/// Lo que Rust mismo observa que está ocurriendo ahora mismo, sin depender de nada que el
/// frontend crea saber.
pub fn close_block_reason(app: &AppHandle) -> Option<&'static str> {
    let guided = crate::commands::guided_in_progress(app);
    let update_state = app
        .try_state::<crate::updates_app::UpdateHandle>()
        .map(|handle| handle.snapshot().state)
        .unwrap_or("idle");
    let busy = app.try_state::<crate::updates_app::BusyOperations>().and_then(|busy| busy.first());
    reason_for(guided, update_state, busy)
}

/// What a `CloseRequested` should actually do, once nothing is blocking it: FR-046's first-close
/// choice, still unmade, needs asking; an already-chosen `tray` should just hide, not ask again;
/// an already-chosen `exit` proceeds — Rust never calls `prevent_close` for that last one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseIntent {
    Blocked(&'static str),
    DecisionRequired,
    Hide,
    Proceed,
}

pub fn intent_for(blocked: Option<&'static str>, close_action: &str) -> CloseIntent {
    if let Some(reason) = blocked {
        return CloseIntent::Blocked(reason);
    }
    match close_action {
        "tray" => CloseIntent::Hide,
        "exit" => CloseIntent::Proceed,
        _ => CloseIntent::DecisionRequired,
    }
}

/// Rust's own read of `lifecycle.close_action`, independent of anything the frontend keeps —
/// falls back to "unset" (ask) exactly like a missing or corrupt value would.
pub fn close_action(app: &AppHandle) -> String {
    let Some(state) = app.try_state::<crate::storage::AppState>() else {
        return "unset".to_owned();
    };
    let Ok(guard) = state.storage.lock() else {
        return "unset".to_owned();
    };
    let Ok(preferences) = guard.user_preferences() else {
        return "unset".to_owned();
    };
    preferences
        .get("lifecycle.close_action")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| "unset".to_owned())
}

pub fn window_close_intent(app: &AppHandle) -> CloseIntent {
    intent_for(close_block_reason(app), &close_action(app))
}

/// Qué debe hacer `confirm_close`, decidido a partir del motivo que Rust calculó (nunca del que
/// afirme el frontend) y de si la persona pidió detener lo que está en curso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmCloseOutcome {
    /// `install` nunca acepta una confirmación: el diálogo solo ofrece «Entendido».
    Denied,
    Proceed {
        stop_guided: bool,
        stop_export: bool,
    },
}

pub fn confirm_close_outcome(
    reason: Option<&'static str>,
    stop_operation: bool,
) -> ConfirmCloseOutcome {
    match reason {
        Some(REASON_INSTALL) => ConfirmCloseOutcome::Denied,
        Some(REASON_GUIDED) if stop_operation => {
            ConfirmCloseOutcome::Proceed { stop_guided: true, stop_export: false }
        }
        Some(REASON_EXPORT) if stop_operation => {
            ConfirmCloseOutcome::Proceed { stop_guided: false, stop_export: true }
        }
        _ => ConfirmCloseOutcome::Proceed { stop_guided: false, stop_export: false },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_running_blocks_nothing() {
        assert_eq!(reason_for(false, "idle", None), None);
    }

    #[test]
    fn an_unset_preference_asks_before_anything_blocking() {
        assert_eq!(intent_for(None, "unset"), CloseIntent::DecisionRequired);
    }

    #[test]
    fn a_saved_tray_preference_hides_instead_of_asking() {
        assert_eq!(intent_for(None, "tray"), CloseIntent::Hide);
    }

    #[test]
    fn a_saved_exit_preference_just_proceeds() {
        assert_eq!(intent_for(None, "exit"), CloseIntent::Proceed);
    }

    #[test]
    fn a_running_operation_wins_over_any_saved_preference() {
        assert_eq!(intent_for(Some(REASON_GUIDED), "exit"), CloseIntent::Blocked(REASON_GUIDED));
        assert_eq!(intent_for(Some(REASON_EXPORT), "tray"), CloseIntent::Blocked(REASON_EXPORT));
    }

    #[test]
    fn an_unrecognised_stored_value_is_treated_like_unset() {
        assert_eq!(intent_for(None, "corrupt"), CloseIntent::DecisionRequired);
    }

    #[test]
    fn a_guided_test_wins_over_everything_else() {
        assert_eq!(reason_for(true, "installing", Some(Blocker::Export)), Some(REASON_GUIDED));
    }

    #[test]
    fn an_install_wins_over_a_download_or_an_export() {
        assert_eq!(reason_for(false, "installing", Some(Blocker::Export)), Some(REASON_INSTALL));
    }

    #[test]
    fn a_download_wins_over_an_export() {
        assert_eq!(reason_for(false, "downloading", Some(Blocker::Export)), Some(REASON_DOWNLOAD));
    }

    #[test]
    fn an_export_or_an_import_both_report_the_export_reason() {
        assert_eq!(reason_for(false, "idle", Some(Blocker::Export)), Some(REASON_EXPORT));
        assert_eq!(reason_for(false, "idle", Some(Blocker::Import)), Some(REASON_EXPORT));
    }

    #[test]
    fn a_data_operation_blocker_does_not_block_closing() {
        assert_eq!(reason_for(false, "idle", Some(Blocker::DataOperation)), None);
    }

    #[test]
    fn an_install_is_always_denied_no_matter_what_was_asked() {
        assert_eq!(confirm_close_outcome(Some(REASON_INSTALL), true), ConfirmCloseOutcome::Denied);
        assert_eq!(confirm_close_outcome(Some(REASON_INSTALL), false), ConfirmCloseOutcome::Denied);
    }

    #[test]
    fn confirming_a_guided_close_stops_the_guided_test_and_nothing_else() {
        assert_eq!(
            confirm_close_outcome(Some(REASON_GUIDED), true),
            ConfirmCloseOutcome::Proceed { stop_guided: true, stop_export: false }
        );
    }

    #[test]
    fn confirming_an_export_close_cancels_the_export() {
        assert_eq!(
            confirm_close_outcome(Some(REASON_EXPORT), true),
            ConfirmCloseOutcome::Proceed { stop_guided: false, stop_export: true }
        );
    }

    #[test]
    fn a_download_never_needs_to_be_stopped_to_close() {
        assert_eq!(
            confirm_close_outcome(Some(REASON_DOWNLOAD), false),
            ConfirmCloseOutcome::Proceed { stop_guided: false, stop_export: false }
        );
    }

    #[test]
    fn nothing_running_by_the_time_it_is_confirmed_just_closes() {
        assert_eq!(
            confirm_close_outcome(None, true),
            ConfirmCloseOutcome::Proceed { stop_guided: false, stop_export: false }
        );
    }
}
