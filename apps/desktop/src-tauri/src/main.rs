// No console window, in any build (2026-09-28): a panic already reaches the log file via the
// panic hook installed in `logging::init` (`RUST_PANIC`, logged before the previous hook runs),
// so nothing depends on this console being visible — it was just an unstyled black window behind
// the app on every `arrancar.ps1` launch. The `e2e` build talks over CDP (port 9222), never stdio
// (`global-setup.ts` spawns it with `stdio: 'ignore'`), so it needs no console either.
#![windows_subsystem = "windows"]

fn main() {
    throttlewatch_lib::run();
}
