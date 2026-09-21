/**
 * Mirrors the backend's `lifecycle::confirm_close_outcome` (`src-tauri/src/lifecycle.rs`): which
 * reasons need `confirm_close` to actually stop something before the window closes. Rust decides
 * whether the close is allowed at all (`install` never is, regardless of this); this only picks
 * the `stop_operation` flag the confirm button sends.
 */

export type CloseBlockedReason = 'guided' | 'export' | 'download' | 'install';

export function stopOperationFor(reason: CloseBlockedReason): boolean {
  return reason === 'guided' || reason === 'export';
}
