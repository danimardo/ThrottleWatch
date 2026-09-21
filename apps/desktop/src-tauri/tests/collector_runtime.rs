//! T164 integration: the real sidecar process feeds the live state through the real runtime
//! (verified spawn, handshake, per-core catalog, samples, orderly stop). It needs the sidecar build
//! (`dotnet build apps/sensor-agent/SensorAgent.sln`); CI builds it before the Rust steps.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use throttlewatch_lib::diagnostics::Ruleset;
use throttlewatch_lib::ipc::supervisor::sha256_hex;
use throttlewatch_lib::telemetry::live::{CollectorState, LiveState};
use throttlewatch_lib::telemetry::runtime::{
    Change, CollectorRuntime, LiveObserver, RuntimeConfig, SidecarLauncher,
};

struct SampleCounter(Arc<AtomicUsize>);

impl LiveObserver for SampleCounter {
    fn updated(&self, change: Change, _live: &LiveState) {
        if change == Change::Sample {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn sidecar() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../sensor-agent/bin/Debug/net10.0-windows/SensorAgent.exe");
    path.is_file().then_some(path)
}

#[test]
fn the_real_sidecar_feeds_the_live_state_and_stops_cleanly() {
    let Some(executable) = sidecar() else {
        assert!(
            std::env::var_os("CI").is_none(),
            "the sidecar build is missing in CI: build apps/sensor-agent first"
        );
        return; // local run without the .NET build: nothing to integrate against
    };
    let bytes = std::fs::read(&executable).unwrap_or_else(|error| panic!("read sidecar: {error}"));
    let launcher = SidecarLauncher { expected_sha256: sha256_hex(&bytes), executable };
    let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));
    let config = RuntimeConfig::from_ruleset(&rules).unwrap_or_else(|| panic!("config"));
    config.interval_ms.store(250, std::sync::atomic::Ordering::SeqCst);

    let live = Arc::new(Mutex::new(LiveState::new()));
    let samples = Arc::new(AtomicUsize::new(0));
    let mut runtime = CollectorRuntime::start(
        Box::new(launcher),
        Arc::clone(&live),
        Arc::new(SampleCounter(Arc::clone(&samples))),
        config,
    )
    .unwrap_or_else(|error| panic!("start: {error}"));

    let deadline = Instant::now() + Duration::from_secs(30);
    while samples.load(Ordering::SeqCst) < 3 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }

    {
        let state = live.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            samples.load(Ordering::SeqCst) >= 3,
            "no samples within 30 s: {:?} {:?}",
            state.collector(),
            state.message_key()
        );
        assert_eq!(state.collector(), CollectorState::Running);
        let cpu = state.cpu().unwrap_or_else(|| panic!("the catalog must have arrived"));
        assert!(!cpu.display_name.is_empty() && cpu.logical_processors > 0);
        assert!(state.snapshot_input().is_some());
        // Whatever this host can read, nothing unreadable is ever published as a number.
        let input = state.snapshot_input().unwrap_or_else(|| panic!("snapshot"));
        for value in [input.temperature_c, input.active_clock_mhz, input.package_power_w]
            .into_iter()
            .flatten()
        {
            assert!(value > 0.0, "a zero reading must be absent, not a value");
        }
        if !cpu.virtualized {
            // Per-core detail was negotiated: the catalog carries one entry per core.
            assert!(!state.cores().is_empty(), "no per-core sensors on a physical host");
        }
    }

    let stopping = Instant::now();
    runtime.stop();
    assert!(stopping.elapsed() < Duration::from_secs(10), "stop must not hang");
    let state = live.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.collector(), CollectorState::Stopped);
}
