use serde_json::Value;

const HANDSHAKE: &str =
    include_str!("../../../../../../packages/contracts/fixtures/handshake.json");
const INVALID_SAMPLE: &str =
    include_str!("../../../../../../packages/contracts/fixtures/invalid-sample.json");

#[test]
fn canonical_handshake_deserializes_as_an_ipc_envelope() {
    let value = match serde_json::from_str::<Value>(HANDSHAKE) {
        Ok(value) => value,
        Err(error) => panic!("fixture must be valid JSON: {error}"),
    };

    assert_eq!(value.get("protocol_version"), Some(&Value::from(1)));
    assert_eq!(value.get("type").and_then(Value::as_str), Some("hello"));
    assert!(value.get("payload").is_some_and(Value::is_object));
}

#[test]
fn invalid_sample_is_rejected_by_the_value_shape_guard() {
    let value = match serde_json::from_str::<Value>(INVALID_SAMPLE) {
        Ok(value) => value,
        Err(error) => panic!("fixture must be valid JSON: {error}"),
    };
    let sample = value
        .get("payload")
        .and_then(|payload| payload.get("values"))
        .and_then(Value::as_array)
        .and_then(|values| values.first());

    assert!(
        sample.is_some_and(|entry| {
            entry.get("number").is_some() && entry.get("boolean").is_some()
        })
    );
}
