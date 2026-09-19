use std::{fs, path::PathBuf};

use super::protocol::validate_message;

fn fixture_paths() -> Vec<PathBuf> {
    let fixture_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/contracts/fixtures");
    let mut paths = fs::read_dir(fixture_dir)
        .unwrap_or_else(|error| panic!("contract fixture directory must exist: {error}"))
        .map(|entry| {
            entry.unwrap_or_else(|error| panic!("fixture entry must be readable: {error}")).path()
        })
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

#[test]
fn all_contract_fixtures_have_the_same_acceptance_result() {
    let paths = fixture_paths();
    assert_eq!(paths.len(), 6, "the conformance corpus must contain six fixtures");

    for path in paths {
        let raw =
            fs::read(&path).unwrap_or_else(|error| panic!("fixture must be readable: {error}"));
        let value = serde_json::from_slice::<serde_json::Value>(&raw)
            .unwrap_or_else(|error| panic!("fixture must be valid JSON: {error}"));
        let nonce = value
            .get("session_nonce")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("fixture must contain a session nonce"));
        let accepted = validate_message(&raw, nonce, None).is_ok();
        let expected = !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("invalid-"));
        assert_eq!(accepted, expected, "fixture {:?}", path.file_name());
    }
}
