#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    Ac,
    Battery,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerContextSnapshot {
    pub source: PowerSource,
    pub battery_percent: Option<u8>,
    pub power_scheme_hash: Option<String>,
    pub resumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerFrameContext {
    pub context: PowerContextSnapshot,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerTransition {
    NoChange,
    Changed,
    Resumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerBroadcastMessage {
    ResumeAutomatic,
    ResumeSuspend,
    StatusChanged,
}

#[derive(Debug, Clone, Copy)]
pub struct PowerContextTracker {
    last_observed_ms: Option<u64>,
    last_source: Option<PowerSource>,
    resume_pending: bool,
}

impl PowerContextTracker {
    pub const fn new() -> Self {
        Self { last_observed_ms: None, last_source: None, resume_pending: false }
    }

    pub fn observe(
        &mut self,
        observed_at_ms: u64,
        source: PowerSource,
        resumed: bool,
    ) -> PowerTransition {
        let transition = if resumed
            || self.resume_pending
            || self
                .last_observed_ms
                .is_some_and(|previous| observed_at_ms.saturating_sub(previous) > 60_000)
        {
            PowerTransition::Resumed
        } else if self.last_source.is_some_and(|previous| previous != source) {
            PowerTransition::Changed
        } else {
            PowerTransition::NoChange
        };
        self.resume_pending = false;
        self.last_observed_ms = Some(observed_at_ms);
        self.last_source = Some(source);
        transition
    }

    pub fn observe_broadcast(&mut self, message: PowerBroadcastMessage, observed_at_ms: u64) {
        if matches!(
            message,
            PowerBroadcastMessage::ResumeAutomatic | PowerBroadcastMessage::ResumeSuspend
        ) {
            self.last_observed_ms = Some(observed_at_ms);
            self.resume_pending = true;
        }
    }
}

impl Default for PowerContextTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
pub fn read_windows_power_context() -> Option<PowerContextSnapshot> {
    let mut raw = RawSystemPowerStatus {
        ac_line_status: 0,
        battery_flag: 0,
        battery_percent: 255,
        reserved: 0,
        battery_life_time: 0,
        battery_full_life_time: 0,
    };
    // SAFETY: the Windows API writes exactly the documented fixed-size status structure.
    let status_ok = unsafe { get_system_power_status(&mut raw) } != 0;
    if !status_ok {
        return None;
    }
    let source = match raw.ac_line_status {
        0 => PowerSource::Battery,
        1 => PowerSource::Ac,
        _ => PowerSource::Unknown,
    };
    let battery_percent = (raw.battery_percent != 255).then_some(raw.battery_percent as i32);
    let scheme_hash = read_active_scheme_hash();
    Some(normalize_power_context(source, battery_percent, scheme_hash, false))
}

#[cfg(windows)]
fn read_active_scheme_hash() -> Option<String> {
    let mut guid_ptr = std::ptr::null_mut();
    // SAFETY: Windows initializes the returned GUID pointer and the matching LocalFree releases it.
    let status = unsafe { power_get_active_scheme(std::ptr::null(), &mut guid_ptr) };
    if status != 0 || guid_ptr.is_null() {
        return None;
    }
    // SAFETY: a successful PowerGetActiveScheme call returns a valid GUID allocation.
    let guid = unsafe { *guid_ptr };
    // SAFETY: the pointer was allocated by the Windows power API and must be freed with LocalFree.
    unsafe { local_free(guid_ptr.cast()) };
    Some(format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    ))
}

#[cfg(windows)]
#[repr(C)]
struct RawSystemPowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_percent: u8,
    reserved: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct WindowsGuid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    #[link_name = "GetSystemPowerStatus"]
    fn get_system_power_status(status: *mut RawSystemPowerStatus) -> i32;
}

#[cfg(windows)]
#[link(name = "PowrProf")]
unsafe extern "system" {
    #[link_name = "PowerGetActiveScheme"]
    fn power_get_active_scheme(
        user_power_key: *const WindowsGuid,
        active_scheme: *mut *mut WindowsGuid,
    ) -> u32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    #[link_name = "LocalFree"]
    fn local_free(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

pub fn normalize_power_context(
    source: PowerSource,
    battery_percent: Option<i32>,
    power_scheme_hash: Option<String>,
    resumed: bool,
) -> PowerContextSnapshot {
    PowerContextSnapshot {
        source,
        battery_percent: battery_percent
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value <= 100),
        power_scheme_hash,
        resumed,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PowerBroadcastMessage, PowerContextTracker, PowerSource, PowerTransition,
        normalize_power_context,
    };

    #[test]
    fn rejects_invalid_battery_percentages_and_preserves_resume_signal() {
        let context =
            normalize_power_context(PowerSource::Battery, Some(140), Some("plan".to_owned()), true);
        assert_eq!(context.battery_percent, None);
        assert!(context.resumed);
    }

    #[test]
    fn detects_resume_and_energy_changes_for_frame_partitioning() {
        let mut tracker = PowerContextTracker::new();
        assert_eq!(tracker.observe(0, PowerSource::Ac, false), PowerTransition::NoChange);
        assert_eq!(tracker.observe(1_000, PowerSource::Battery, false), PowerTransition::Changed);
        assert_eq!(tracker.observe(70_000, PowerSource::Battery, false), PowerTransition::Resumed);
    }

    #[test]
    fn treats_resume_broadcasts_as_a_new_sampling_epoch() {
        let mut tracker = PowerContextTracker::default();
        tracker.observe_broadcast(PowerBroadcastMessage::ResumeAutomatic, 10_000);
        assert_eq!(tracker.observe(10_000, PowerSource::Ac, false), PowerTransition::Resumed);
    }
}
