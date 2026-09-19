use serde_json::Value;
use std::fmt::{Display, Formatter};

pub const PROTOCOL_VERSION: u64 = 1;
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum ProtocolError {
    MessageTooLarge,
    InvalidJson,
    InvalidEnvelope(&'static str),
    UnsupportedVersion(u64),
    NonceMismatch,
    SequenceRegression { previous: u64, received: u64 },
    SequenceGap { expected: u64, received: u64 },
}

fn is_known_message_type(message_type: &str) -> bool {
    matches!(
        message_type,
        "hello"
            | "hello_ack"
            | "capabilities"
            | "start"
            | "started"
            | "set_rate"
            | "snapshot"
            | "sample"
            | "stop"
            | "stopped"
            | "shutdown"
            | "error"
            | "elevated_start"
    )
}

fn has_exactly_one_value(entry: &serde_json::Map<String, Value>) -> bool {
    let has_number = entry.get("number").is_some_and(Value::is_number);
    let has_boolean = entry.get("boolean").is_some_and(Value::is_boolean);
    has_number ^ has_boolean
}

fn valid_sample_payload(payload: &Value) -> bool {
    let Some(object) = payload.as_object() else {
        return false;
    };
    let Some(values) = object.get("values").and_then(Value::as_array) else {
        return false;
    };
    if object.get("monotonic_ms").and_then(Value::as_u64).is_none()
        || object
            .get("duration_ms")
            .and_then(Value::as_u64)
            .is_none_or(|duration| duration > 10_000)
    {
        return false;
    }

    values.iter().all(|value| {
        let Some(entry) = value.as_object() else {
            return false;
        };
        let Some(status) = entry.get("status").and_then(Value::as_str) else {
            return false;
        };
        if entry.get("sensor_id").and_then(Value::as_str).is_none_or(str::is_empty) {
            return false;
        }
        match status {
            "ok" => has_exactly_one_value(entry),
            "missing" | "stale" | "invalid" | "unsupported" => {
                !entry.contains_key("number") && !entry.contains_key("boolean")
            }
            _ => false,
        }
    })
}

fn valid_payload(message_type: &str, payload: &Value) -> bool {
    let Some(object) = payload.as_object() else {
        return false;
    };
    match message_type {
        "hello" => {
            object
                .get("app_version")
                .and_then(Value::as_str)
                .is_some_and(|version| !version.is_empty())
                && object.get("supported_protocols").and_then(Value::as_array).is_some_and(
                    |protocols| protocols.iter().any(|protocol| protocol.as_u64() == Some(1)),
                )
        }
        "capabilities" => {
            object.get("cpu").is_some_and(Value::is_object)
                && object.get("groups").is_some_and(Value::is_array)
                && object.get("sensors").is_some_and(Value::is_array)
        }
        "sample" => valid_sample_payload(payload),
        "error" => {
            object.get("code").and_then(Value::as_str).is_some_and(|code| {
                !code.is_empty()
                    && code.chars().all(|character| {
                        character.is_ascii_uppercase()
                            || character.is_ascii_digit()
                            || character == '_'
                    })
            }) && object
                .get("severity")
                .and_then(Value::as_str)
                .is_some_and(|severity| matches!(severity, "info" | "recoverable" | "fatal"))
                && object
                    .get("message_key")
                    .and_then(Value::as_str)
                    .is_some_and(|message_key| !message_key.is_empty())
        }
        _ => true,
    }
}

impl Display for ProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageTooLarge => formatter.write_str("message exceeds the protocol size limit"),
            Self::InvalidJson => formatter.write_str("message is not valid JSON"),
            Self::InvalidEnvelope(field) => write!(formatter, "invalid envelope field: {field}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported protocol version: {version}")
            }
            Self::NonceMismatch => formatter.write_str("session nonce does not match"),
            Self::SequenceRegression { previous, received } => {
                write!(formatter, "sequence regressed from {previous} to {received}")
            }
            Self::SequenceGap { expected, received } => {
                write!(formatter, "sequence gap: expected {expected}, received {received}")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValidatedEnvelope {
    pub message_type: String,
    pub sequence: u64,
}

pub fn validate_message(
    raw: &[u8],
    expected_nonce: &str,
    previous_sequence: Option<u64>,
) -> Result<ValidatedEnvelope, ProtocolError> {
    if raw.len() > MAX_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge);
    }

    let value = serde_json::from_slice::<Value>(raw).map_err(|_| ProtocolError::InvalidJson)?;
    let object = value.as_object().ok_or(ProtocolError::InvalidEnvelope("object"))?;
    let version = object
        .get("protocol_version")
        .and_then(Value::as_u64)
        .ok_or(ProtocolError::InvalidEnvelope("protocol_version"))?;
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(version));
    }

    let nonce = object
        .get("session_nonce")
        .and_then(Value::as_str)
        .ok_or(ProtocolError::InvalidEnvelope("session_nonce"))?;
    if nonce != expected_nonce {
        return Err(ProtocolError::NonceMismatch);
    }

    let sequence = object
        .get("sequence")
        .and_then(Value::as_u64)
        .ok_or(ProtocolError::InvalidEnvelope("sequence"))?;
    if let Some(previous) = previous_sequence {
        if sequence <= previous {
            return Err(ProtocolError::SequenceRegression { previous, received: sequence });
        }
        if sequence != previous.saturating_add(1) {
            return Err(ProtocolError::SequenceGap {
                expected: previous.saturating_add(1),
                received: sequence,
            });
        }
    }

    let message_type = object
        .get("type")
        .and_then(Value::as_str)
        .filter(|message_type| !message_type.is_empty())
        .ok_or(ProtocolError::InvalidEnvelope("type"))?;
    if !is_known_message_type(message_type)
        || !object
            .get("payload")
            .is_some_and(|payload| payload.is_object() && valid_payload(message_type, payload))
    {
        return Err(ProtocolError::InvalidEnvelope("payload"));
    }

    Ok(ValidatedEnvelope { message_type: message_type.to_owned(), sequence })
}

#[cfg(test)]
mod tests {
    use super::{ProtocolError, validate_message};

    const MESSAGE: &[u8] = br#"{"protocol_version":1,"session_nonce":"nonce","sequence":2,"timestamp_utc":"2026-09-18T10:00:00Z","type":"sample","payload":{"monotonic_ms":1,"duration_ms":1,"values":[]}}"#;

    #[test]
    fn accepts_matching_nonce_and_increasing_sequence() {
        let result = validate_message(MESSAGE, "nonce", Some(1));
        assert_eq!(
            result,
            Ok(super::ValidatedEnvelope { message_type: "sample".to_owned(), sequence: 2 })
        );
    }

    #[test]
    fn rejects_nonce_mismatch_and_sequence_regression() {
        assert_eq!(validate_message(MESSAGE, "other", Some(1)), Err(ProtocolError::NonceMismatch));
        assert_eq!(
            validate_message(MESSAGE, "nonce", Some(2)),
            Err(ProtocolError::SequenceRegression { previous: 2, received: 2 })
        );
    }

    #[test]
    fn rejects_messages_over_one_megabyte() {
        let mut message = MESSAGE.to_vec();
        message.resize(super::MAX_MESSAGE_BYTES + 1, b' ');
        assert_eq!(validate_message(&message, "nonce", None), Err(ProtocolError::MessageTooLarge));
    }
}
