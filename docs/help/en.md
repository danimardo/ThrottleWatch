# ThrottleWatch help

## What it measures

ThrottleWatch observes temperature, load, active frequency and package power. It is not a
benchmark and it does not change the computer's limits. Values carry the quality of the available
signal and the interface formats times in the active locale.

- **Temperature:** CPU package temperature and the observed margin to the thermal limit.
- **Active frequency:** an estimate of effective frequency while work is running, not a marketing
  clock or a raw multiplier reading.
- **Load:** aggregated activity across logical processors for the selected window.
- **Power:** package power when the hardware exposes a direct reading; derived readings are marked.

## Coverage and confidence

- **Tier A:** enough direct signals are available to confirm a limit for the session's scope.
- **Tier B:** advanced signals are missing; the app can explain likely patterns but does not claim
  confirmation that needs those signals.
- **Tier C:** essential data is missing or the collector is disconnected; the result is
  insufficient and no numbers are invented.

Coverage is session-scoped. If advanced access fails during a session, later samples degrade to
B/C and A conclusions are not extended across that interval.

## Reading the result

A confirmed thermal limit combines temperature near its limit with a frequency drop in the same
window. A power limit shows sustained power at the effective ceiling without reaching the thermal
limit. Platform management can resemble power limiting, so the app presents it as a platform
hypothesis when direct signals are unavailable.

“Improve cooling” is a cautious recommendation, not a promised percentage. A quantified cooling
potential is shown only when tier A stays intact for the whole required window.

## Guided diagnostic

The guided diagnostic observes idle, warm-up, sustained-load and recovery phases. The current
version does not generate an integrated CPU workload. `Ctrl+Shift+X` stops it; reaching a thermal
limit alone is never a stop command. Hiding or closing the window cancels the run safely.

## Advanced access and privacy

PawnIO is a shared machine driver and is not removed when ThrottleWatch is uninstalled. Tier A
may require an administrator account during installation; the daily interface remains
unprivileged. Tier B/C diagnostics remain available if UAC is refused or the provider is absent.

Sessions are stored locally. Anonymous export removes the identifiers defined by the contract and
validates the document again before writing it. Detailed logging is opt-in and returns to normal
on restart or after 24 hours.

## Shortcuts

`Ctrl+1…Ctrl+6` navigates screens, `Ctrl+,` opens Settings, `Ctrl+E` exports the active session,
`F1` opens this help, `Ctrl+Shift+X` stops the guided diagnostic, and `Esc` closes dialogs or
tooltips.

