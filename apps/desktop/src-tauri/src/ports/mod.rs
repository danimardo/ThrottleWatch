use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortError {
    pub code: &'static str,
}

pub trait ClockPort {
    fn monotonic_ms(&self) -> u64;
    fn utc_timestamp(&self) -> String;
}

pub trait IdPort {
    fn new_id(&mut self) -> String;
}

pub trait NoncePort {
    fn new_nonce(&mut self) -> String;
}

pub trait StoragePort {
    fn delete_session(&mut self, session_id: &str) -> Result<(), PortError>;
}

pub trait DialogPort {
    fn open_file(&mut self) -> Result<Option<PathBuf>, PortError>;
    fn save_file(&mut self) -> Result<Option<PathBuf>, PortError>;
}

pub trait WindowPort {
    fn show(&mut self) -> Result<(), PortError>;
    fn close(&mut self) -> Result<(), PortError>;
}

pub trait TrayPort {
    fn set_paused(&mut self, paused: bool) -> Result<(), PortError>;
}

pub trait NotificationPort {
    fn notify(&mut self, title: &str, body: &str) -> Result<(), PortError>;
}

pub trait AutostartPort {
    fn set_enabled(&mut self, enabled: bool) -> Result<(), PortError>;
}

pub trait HttpPort {
    fn get(&mut self, endpoint_id: &str) -> Result<Vec<u8>, PortError>;
}

pub trait SystemContextPort {
    fn power_source(&self) -> &'static str;
    fn locale(&self) -> &'static str;
    fn theme(&self) -> &'static str;
}
