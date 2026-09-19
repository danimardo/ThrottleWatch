use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;
use throttlewatch_lib::ipc::protocol::{ProtocolError, validate_message};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn replay_process(trace: &str) -> std::process::Child {
    let root = repository_root();
    Command::new("node")
        .current_dir(&root)
        .args(["packages/trace-fixtures/tools/replay.mjs", "--trace", trace])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("replay process must start: {error}"))
}

#[test]
fn replays_a_real_process_and_validates_every_message_until_eof() {
    let mut child = replay_process("packages/trace-fixtures/intel-thermal-confirmed.json");
    let stdout = child.stdout.take().unwrap_or_else(|| panic!("replay stdout must be piped"));
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let mut previous_sequence = None;
    let mut messages = 0;
    let mut nonce = None;

    while reader
        .read_line(&mut line)
        .unwrap_or_else(|error| panic!("replay stdout must be readable: {error}"))
        > 0
    {
        let value = serde_json::from_str::<Value>(&line)
            .unwrap_or_else(|error| panic!("replay emits JSON: {error}"));
        let message_nonce =
            value["session_nonce"].as_str().unwrap_or_else(|| panic!("replay message has a nonce"));
        let expected_nonce = nonce.get_or_insert_with(|| message_nonce.to_owned());
        let validated = validate_message(line.as_bytes(), expected_nonce, previous_sequence);
        let envelope = validated.unwrap_or_else(|error| panic!("replay message is valid: {error}"));
        previous_sequence = Some(envelope.sequence);
        messages += 1;
        line.clear();
    }

    let status = child.wait().unwrap_or_else(|error| panic!("replay process must finish: {error}"));
    assert!(status.success());
    assert_eq!(messages, 2);
}

#[test]
fn rejects_a_real_process_with_an_invalid_protocol_version() {
    let code = "process.stdout.write(JSON.stringify({protocol_version: 2, session_nonce: 'fixture-session-nonce-1', sequence: 0, timestamp_utc: '2026-09-18T10:00:00.000Z', type: 'hello', payload: {}}) + '\\n');";
    let mut child = Command::new("node")
        .current_dir(repository_root())
        .args(["-e", code])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("invalid replay process must start: {error}"));
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap_or_else(|| panic!("invalid replay stdout must be piped"))
        .read_to_string(&mut output)
        .unwrap_or_else(|error| panic!("invalid replay stdout must be readable: {error}"));
    child.wait().unwrap_or_else(|error| panic!("process must finish: {error}"));

    assert_eq!(
        validate_message(output.as_bytes(), "fixture-session-nonce-1", None),
        Err(ProtocolError::UnsupportedVersion(2))
    );
}

#[test]
fn rejects_sequence_gaps_and_stops_a_hung_real_process() {
    let first = br#"{"protocol_version":1,"session_nonce":"fixture-session-nonce-1","sequence":0,"timestamp_utc":"2026-09-18T10:00:00.000Z","type":"hello","payload":{"app_version":"0.1.0","supported_protocols":[1]}}"#;
    let second = br#"{"protocol_version":1,"session_nonce":"fixture-session-nonce-1","sequence":2,"timestamp_utc":"2026-09-18T10:00:00.100Z","type":"capabilities","payload":{"cpu":{},"groups":[],"sensors":[]}}"#;
    let first_result = validate_message(first, "fixture-session-nonce-1", None)
        .unwrap_or_else(|error| panic!("first message is valid: {error}"));
    assert_eq!(
        validate_message(second, "fixture-session-nonce-1", Some(first_result.sequence)),
        Err(ProtocolError::SequenceGap { expected: 1, received: 2 })
    );
    let missing_value = br#"{"protocol_version":1,"session_nonce":"fixture-session-nonce-1","sequence":1,"timestamp_utc":"2026-09-18T10:00:00.050Z","type":"sample","payload":{"monotonic_ms":1,"duration_ms":1,"values":[{"sensor_id":"cpu.package.temp","status":"missing"}]}}"#;
    assert!(validate_message(missing_value, "fixture-session-nonce-1", Some(0)).is_ok());

    let mut hung = Command::new("node")
        .current_dir(repository_root())
        .args(["-e", "setTimeout(() => {}, 10000);"])
        .stdout(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("hung process must start: {error}"));
    assert!(
        hung.try_wait()
            .unwrap_or_else(|error| panic!("hung process can be polled: {error}"))
            .is_none()
    );
    hung.kill().unwrap_or_else(|error| panic!("hung process must be stoppable: {error}"));
    hung.wait().unwrap_or_else(|error| panic!("hung process must be reaped: {error}"));
}
