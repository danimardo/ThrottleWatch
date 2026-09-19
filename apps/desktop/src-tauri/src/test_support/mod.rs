#![cfg(test)]

use crate::ports::{
    AutostartPort, ClockPort, DialogPort, HttpPort, IdPort, NoncePort, NotificationPort, PortError,
    StoragePort, SystemContextPort, TrayPort, WindowPort,
};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct FakePorts {
    pub now_ms: u64,
    pub ids: u64,
    pub nonces: u64,
    pub deleted_sessions: Vec<String>,
    pub paused: bool,
    pub window_visible: bool,
    pub autostart: bool,
}

impl ClockPort for FakePorts {
    fn monotonic_ms(&self) -> u64 {
        self.now_ms
    }
    fn utc_timestamp(&self) -> String {
        "2026-09-18T10:00:00.000Z".to_owned()
    }
}

impl IdPort for FakePorts {
    fn new_id(&mut self) -> String {
        self.ids += 1;
        format!("fake-id-{}", self.ids)
    }
}

impl NoncePort for FakePorts {
    fn new_nonce(&mut self) -> String {
        self.nonces += 1;
        format!("fake-nonce-{}", self.nonces)
    }
}

impl StoragePort for FakePorts {
    fn delete_session(&mut self, session_id: &str) -> Result<(), PortError> {
        self.deleted_sessions.push(session_id.to_owned());
        Ok(())
    }
}

impl DialogPort for FakePorts {
    fn open_file(&mut self) -> Result<Option<PathBuf>, PortError> {
        Ok(None)
    }
    fn save_file(&mut self) -> Result<Option<PathBuf>, PortError> {
        Ok(None)
    }
}

impl WindowPort for FakePorts {
    fn show(&mut self) -> Result<(), PortError> {
        self.window_visible = true;
        Ok(())
    }
    fn close(&mut self) -> Result<(), PortError> {
        self.window_visible = false;
        Ok(())
    }
}

impl TrayPort for FakePorts {
    fn set_paused(&mut self, paused: bool) -> Result<(), PortError> {
        self.paused = paused;
        Ok(())
    }
}

impl NotificationPort for FakePorts {
    fn notify(&mut self, _title: &str, _body: &str) -> Result<(), PortError> {
        Ok(())
    }
}

impl AutostartPort for FakePorts {
    fn set_enabled(&mut self, enabled: bool) -> Result<(), PortError> {
        self.autostart = enabled;
        Ok(())
    }
}

impl HttpPort for FakePorts {
    fn get(&mut self, _endpoint_id: &str) -> Result<Vec<u8>, PortError> {
        Ok(Vec::new())
    }
}

impl SystemContextPort for FakePorts {
    fn power_source(&self) -> &'static str {
        "unknown"
    }
    fn locale(&self) -> &'static str {
        "es-ES"
    }
    fn theme(&self) -> &'static str {
        "dark"
    }
}

#[derive(Debug, Default)]
pub struct TraceBuilder {
    messages: Vec<Value>,
    sequence: u64,
}

impl TraceBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn message(mut self, message_type: &str, payload: Value) -> Self {
        self.messages.push(json!({
            "protocol_version": 1,
            "session_nonce": "test-session-nonce",
            "sequence": self.sequence,
            "timestamp_utc": "2026-09-18T10:00:00.000Z",
            "type": message_type,
            "payload": payload
        }));
        self.sequence += 1;
        self
    }

    pub fn capabilities(self) -> Self {
        self.message(
            "capabilities",
            json!({
                "cpu": {
                    "vendor": "intel",
                    "display_name": "Test CPU",
                    "logical_processors": 8,
                    "physical_cores": 4,
                    "hybrid": false,
                    "virtualized": true
                },
                "groups": [{ "id": "all", "kind": "homogeneous", "logical_count": 8 }],
                "sensors": []
            }),
        )
    }

    pub fn sample(self, temperature_c: f64, thermal_flag: bool) -> Self {
        let monotonic_ms = self.sequence * 1000;
        self.message(
            "sample",
            json!({
                "monotonic_ms": monotonic_ms,
                "duration_ms": 28,
                "values": [
                    { "sensor_id": "cpu.package.temp", "number": temperature_c, "status": "ok" },
                    { "sensor_id": "cpu.thermal.flag", "boolean": thermal_flag, "status": "ok" }
                ]
            }),
        )
    }

    pub fn build(self) -> Vec<Value> {
        self.messages
    }
}

#[cfg(test)]
mod tests {
    use super::{FakePorts, TraceBuilder};
    use crate::ports::{IdPort, NoncePort, StoragePort, SystemContextPort, TrayPort};

    #[test]
    fn builders_create_ordered_capabilities_and_samples() {
        let trace = TraceBuilder::new().capabilities().sample(96.0, true).build();

        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0]["type"], "capabilities");
        assert_eq!(trace[1]["sequence"], 1);
        assert_eq!(trace[1]["payload"]["values"][0]["number"], 96.0);
    }

    #[test]
    fn fake_ports_are_deterministic_and_record_actions() -> Result<(), crate::ports::PortError> {
        let mut ports = FakePorts::default();
        assert_eq!(ports.new_id(), "fake-id-1");
        assert_eq!(ports.new_nonce(), "fake-nonce-1");
        ports.set_paused(true)?;
        ports.delete_session("session-1")?;
        assert!(ports.paused);
        assert_eq!(ports.deleted_sessions, vec!["session-1"]);
        assert_eq!(ports.locale(), "es-ES");
        Ok(())
    }
}
