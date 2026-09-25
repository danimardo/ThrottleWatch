//! Signed release manifest for the elevated launcher (ADR-0004 R2 and C3).
//!
//! The manifest lists the files the launcher may start with privileges and the SHA-256 of each.
//! A process of the same user can rewrite a file *and* an unsigned manifest next to it, but it
//! cannot forge the minisign signature, so the signature is verified first and the JSON is
//! parsed only afterwards: unverified bytes never reach the parser.
//!
//! Verification is never disabled. Debug and `e2e` builds trust the development key
//! (`keys/dev-release.pub`, secret half outside the repository); a release build trusts no key
//! until the updater key exists and therefore rejects every manifest.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use minisign_verify::{Error as MinisignError, PublicKey, Signature};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

/// Public half of the DEVELOPMENT signing key. Never trust it in a release build: use
/// [`trusted_public_key`].
pub const DEV_PUBLIC_KEY: &str = include_str!("../keys/dev-release.pub");

/// Public half of the PRODUCTION signing key (T102): trusted only in release builds, never in
/// debug or `e2e` — those always trust [`DEV_PUBLIC_KEY`] instead, so a developer's machine can
/// never accidentally verify something signed with the production secret half, and a release
/// build can never be fooled by the development key. The secret half lives only as the
/// `UPDATER_SIGNING_KEY` GitHub Actions secret (`.github/workflows/release.yml`) — nowhere in
/// this repository, nowhere on a developer's machine.
pub const PRODUCTION_PUBLIC_KEY: &str = include_str!("../keys/updater-release.pub");

const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const SUPPORTED_MANIFEST_VERSION: u32 = 1;

/// Public key this build trusts: the development key in debug, `e2e` and `dev-signing` builds,
/// the production key otherwise. Never `None` since T102 — before it, a release build trusted
/// nothing at all.
///
/// `dev-signing` (T112) exists because the production secret lives only in the
/// `UPDATER_SIGNING_KEY` GitHub secret, so a locally built installer could never start a
/// collector: its bundled manifest can only be signed with the development key. The feature is
/// off by default and `release.yml` never passes it, so a real release trusts the production key
/// by construction. [`built_for_testing_only`] lets the application say so out loud at startup.
pub fn trusted_public_key() -> Option<&'static str> {
    if cfg!(any(debug_assertions, feature = "e2e", feature = "dev-signing")) {
        Some(DEV_PUBLIC_KEY)
    } else {
        Some(PRODUCTION_PUBLIC_KEY)
    }
}

/// True when this is an optimised build that nonetheless trusts the development key — an
/// installer meant for testing, never for release. Always false in a real release.
pub fn built_for_testing_only() -> bool {
    !cfg!(debug_assertions) && cfg!(any(feature = "e2e", feature = "dev-signing"))
}

#[derive(Debug)]
pub enum ManifestError {
    TooLarge,
    InvalidPublicKey,
    InvalidSignatureEncoding,
    /// Wrong key, tampered manifest or signature of something else.
    SignatureRejected,
    InvalidJson,
    UnsupportedVersion,
    InvalidEntry(&'static str),
    DuplicatePath,
    NotListed,
    HashMismatch,
    Io(io::Error),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("manifest is too large"),
            Self::InvalidPublicKey => formatter.write_str("public key is not valid minisign"),
            Self::InvalidSignatureEncoding => {
                formatter.write_str("signature is not valid minisign")
            }
            Self::SignatureRejected => formatter.write_str("manifest signature was rejected"),
            Self::InvalidJson => formatter.write_str("manifest is not valid"),
            Self::UnsupportedVersion => formatter.write_str("manifest version is not supported"),
            Self::InvalidEntry(reason) => {
                write!(formatter, "manifest entry is not valid: {reason}")
            }
            Self::DuplicatePath => formatter.write_str("manifest lists a path twice"),
            Self::NotListed => formatter.write_str("file is not listed in the manifest"),
            Self::HashMismatch => formatter.write_str("file hash does not match the manifest"),
            Self::Io(_) => formatter.write_str("file could not be read"),
        }
    }
}

impl std::error::Error for ManifestError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    manifest_version: u32,
    version: String,
    files: Vec<RawFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    /// Relative to the install directory; only plain path components.
    pub path: PathBuf,
    /// Lowercase hexadecimal SHA-256.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub version: String,
    pub files: Vec<ManifestFile>,
}

impl ReleaseManifest {
    /// Verifies the minisign `signature` of `manifest` against `public_key` and only then parses it.
    pub fn verify(
        manifest: &[u8],
        signature: &str,
        public_key: &str,
    ) -> Result<Self, ManifestError> {
        if manifest.len() > MAX_MANIFEST_BYTES {
            return Err(ManifestError::TooLarge);
        }
        let key = PublicKey::decode(public_key).map_err(|_| ManifestError::InvalidPublicKey)?;
        let signature =
            Signature::decode(signature).map_err(|_| ManifestError::InvalidSignatureEncoding)?;
        // `false`: only the pre-hashed variant that current minisign and rsign2 emit.
        key.verify(manifest, &signature, false).map_err(|error| match error {
            MinisignError::InvalidEncoding | MinisignError::UnsupportedAlgorithm => {
                ManifestError::InvalidSignatureEncoding
            }
            _ => ManifestError::SignatureRejected,
        })?;
        Self::from_verified_bytes(manifest)
    }

    /// Parses and validates manifest content. Call it only on bytes whose signature was verified.
    pub(crate) fn from_verified_bytes(manifest: &[u8]) -> Result<Self, ManifestError> {
        let raw: RawManifest =
            serde_json::from_slice(manifest).map_err(|_| ManifestError::InvalidJson)?;
        if raw.manifest_version != SUPPORTED_MANIFEST_VERSION {
            return Err(ManifestError::UnsupportedVersion);
        }
        if raw.version.trim().is_empty() {
            return Err(ManifestError::InvalidEntry("empty version"));
        }
        if raw.files.is_empty() {
            return Err(ManifestError::InvalidEntry("no files"));
        }
        let mut files: Vec<ManifestFile> = Vec::with_capacity(raw.files.len());
        for entry in raw.files {
            let path = validate_relative_path(&entry.path)?;
            let sha256 = validate_sha256(&entry.sha256)?;
            let duplicate = files.iter().any(|existing| {
                existing.path.to_string_lossy().eq_ignore_ascii_case(&path.to_string_lossy())
            });
            if duplicate {
                return Err(ManifestError::DuplicatePath);
            }
            files.push(ManifestFile { path, sha256 });
        }
        Ok(Self { version: raw.version, files })
    }

    /// Expected SHA-256 of the listed file, if any.
    pub fn sha256_for(&self, relative: &Path) -> Option<&str> {
        self.files
            .iter()
            .find(|file| {
                file.path.to_string_lossy().eq_ignore_ascii_case(&relative.to_string_lossy())
            })
            .map(|file| file.sha256.as_str())
    }

    /// Hashes `install_dir/relative` and compares it with the manifest. The caller must still start
    /// the process from the file it has verified (see `supervisor::spawn_verified`), not from a
    /// path resolved again later.
    pub fn check_file(&self, install_dir: &Path, relative: &Path) -> Result<(), ManifestError> {
        let expected = self.sha256_for(relative).ok_or(ManifestError::NotListed)?;
        let actual = sha256_file(&install_dir.join(relative)).map_err(ManifestError::Io)?;
        if actual == expected { Ok(()) } else { Err(ManifestError::HashMismatch) }
    }
}

fn validate_relative_path(value: &str) -> Result<PathBuf, ManifestError> {
    if value.is_empty() {
        return Err(ManifestError::InvalidEntry("empty path"));
    }
    // ':' rejects drive letters and NTFS alternate data streams.
    if value.contains(':') {
        return Err(ManifestError::InvalidEntry("path contains ':'"));
    }
    let path = PathBuf::from(value);
    if path.is_absolute() || path.has_root() {
        return Err(ManifestError::InvalidEntry("path is not relative"));
    }
    if !path.components().all(|component| matches!(component, Component::Normal(_))) {
        return Err(ManifestError::InvalidEntry("path is not a plain relative path"));
    }
    Ok(path)
}

fn validate_sha256(value: &str) -> Result<String, ManifestError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ManifestError::InvalidEntry("sha256 is not 64 hexadecimal characters"));
    }
    Ok(value.to_ascii_lowercase())
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &[u8] =
        include_bytes!("../tests/fixtures/release-manifest/release-manifest.json");
    const SIGNATURE: &str =
        include_str!("../tests/fixtures/release-manifest/release-manifest.json.minisig");
    const OTHER_KEY_SIGNATURE: &str =
        include_str!("../tests/fixtures/release-manifest/release-manifest.json.other.minisig");
    const OTHER_PUBLIC_KEY: &str = include_str!("../tests/fixtures/release-manifest/other.pub");

    fn fixture_sha256() -> String {
        let digest = Sha256::digest(b"sidecar-fixture");
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tw-release-manifest-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("scratch dir: {error}"));
        dir
    }

    fn parse(json: &str) -> Result<ReleaseManifest, ManifestError> {
        ReleaseManifest::from_verified_bytes(json.as_bytes())
    }

    fn manifest_with_path(path: &str) -> String {
        format!(
            r#"{{"manifest_version":1,"version":"1.0.0","files":[{{"path":{path:?},"sha256":"{}"}}]}}"#,
            fixture_sha256()
        )
    }

    #[test]
    fn accepts_the_manifest_signed_with_the_development_key() {
        let manifest = ReleaseManifest::verify(MANIFEST, SIGNATURE, DEV_PUBLIC_KEY)
            .unwrap_or_else(|error| panic!("fixture must verify: {error}"));
        assert_eq!(manifest.version, "0.0.0-fixture");
        assert_eq!(
            manifest.sha256_for(Path::new("SensorAgent.exe")),
            Some(fixture_sha256().as_str())
        );
    }

    #[test]
    fn rejects_a_manifest_signed_with_a_different_key() {
        let result = ReleaseManifest::verify(MANIFEST, OTHER_KEY_SIGNATURE, DEV_PUBLIC_KEY);
        assert!(matches!(result, Err(ManifestError::SignatureRejected)), "{result:?}");
    }

    #[test]
    fn rejects_the_development_signature_under_another_public_key() {
        let result = ReleaseManifest::verify(MANIFEST, SIGNATURE, OTHER_PUBLIC_KEY);
        assert!(matches!(result, Err(ManifestError::SignatureRejected)), "{result:?}");
    }

    #[test]
    fn rejects_a_tampered_manifest() {
        let tampered =
            String::from_utf8_lossy(MANIFEST).replace("SensorAgent.exe", "Sensor2Agent.exe");
        let result = ReleaseManifest::verify(tampered.as_bytes(), SIGNATURE, DEV_PUBLIC_KEY);
        assert!(matches!(result, Err(ManifestError::SignatureRejected)), "{result:?}");
    }

    #[test]
    fn rejects_malformed_key_signature_and_oversized_manifest() {
        assert!(matches!(
            ReleaseManifest::verify(MANIFEST, SIGNATURE, "not a key"),
            Err(ManifestError::InvalidPublicKey)
        ));
        assert!(matches!(
            ReleaseManifest::verify(MANIFEST, "not a signature", DEV_PUBLIC_KEY),
            Err(ManifestError::InvalidSignatureEncoding)
        ));
        let oversized = vec![b' '; MAX_MANIFEST_BYTES + 1];
        assert!(matches!(
            ReleaseManifest::verify(&oversized, SIGNATURE, DEV_PUBLIC_KEY),
            Err(ManifestError::TooLarge)
        ));
    }

    #[test]
    fn test_builds_trust_the_development_key_not_the_production_one() {
        // `cargo test` always compiles with `debug_assertions` on, so this can only exercise the
        // development branch — the production branch needs an actual release compile to flip
        // `cfg!(debug_assertions)`, which `production_and_development_keys_are_not_the_same_key`
        // below at least keeps honest without one.
        assert_eq!(trusted_public_key(), Some(DEV_PUBLIC_KEY));
    }

    #[test]
    fn a_build_without_dev_signing_never_trusts_the_development_key() {
        // The property T102 bought and T112 must not sell: an optimised build with no explicit
        // testing feature trusts the production key only. Meaningless under `cargo test` alone
        // (always debug), so it is exercised on purpose with `cargo test --release`, and again
        // with `--release --features dev-signing` to see the other branch.
        if cfg!(any(debug_assertions, feature = "e2e", feature = "dev-signing")) {
            return;
        }
        assert_eq!(trusted_public_key(), Some(PRODUCTION_PUBLIC_KEY));
        assert_ne!(trusted_public_key(), Some(DEV_PUBLIC_KEY));
    }

    #[test]
    fn dev_signing_swaps_the_trusted_key_and_flags_the_build() {
        if !cfg!(feature = "dev-signing") || cfg!(debug_assertions) {
            return;
        }
        assert_eq!(trusted_public_key(), Some(DEV_PUBLIC_KEY));
        assert!(super::built_for_testing_only(), "a test installer must announce itself");
    }

    #[test]
    fn a_debug_build_is_not_flagged_as_a_test_only_installer() {
        // `built_for_testing_only` is about an *optimised* build that trusts the development key
        // (T112's `dev-signing`). A debug build already trusts it for other reasons and is never
        // distributed, so flagging it would only cry wolf on every developer's run.
        if !cfg!(debug_assertions) {
            return; // `cargo test --release` is exactly the case this test must not judge.
        }
        assert!(!super::built_for_testing_only());
    }

    #[test]
    fn production_and_development_keys_are_not_the_same_key() {
        // T102: if these ever matched, a debug build's manifest would also verify against the
        // production key, and vice versa — the whole point of having two.
        assert_ne!(DEV_PUBLIC_KEY, PRODUCTION_PUBLIC_KEY);
        assert!(
            PublicKey::decode(PRODUCTION_PUBLIC_KEY).is_ok(),
            "keys/updater-release.pub must be a real minisign public key, not a placeholder"
        );
    }

    #[test]
    fn rejects_unknown_fields_and_unsupported_versions() {
        let unknown =
            manifest_with_path("SensorAgent.exe").replace("\"version\"", "\"extra\":1,\"version\"");
        assert!(matches!(parse(&unknown), Err(ManifestError::InvalidJson)));
        let future = manifest_with_path("SensorAgent.exe")
            .replace("\"manifest_version\":1", "\"manifest_version\":2");
        assert!(matches!(parse(&future), Err(ManifestError::UnsupportedVersion)));
        assert!(matches!(parse("{"), Err(ManifestError::InvalidJson)));
    }

    #[test]
    fn rejects_paths_that_could_leave_the_install_directory() {
        for path in [
            "",
            "..\\evil.exe",
            "a\\..\\b.exe",
            "./SensorAgent.exe",
            "C:\\Windows\\System32\\cmd.exe",
            "\\\\server\\share\\x.exe",
            "\\Windows\\x.exe",
            "/etc/x",
            "SensorAgent.exe:stream",
        ] {
            let result = parse(&manifest_with_path(path));
            assert!(
                matches!(result, Err(ManifestError::InvalidEntry(_))),
                "{path:?} -> {result:?}"
            );
        }
        assert!(parse(&manifest_with_path("bin\\SensorAgent.exe")).is_ok());
    }

    #[test]
    fn rejects_bad_hashes_duplicates_and_empty_lists() {
        let short = manifest_with_path("a.exe").replace(&fixture_sha256(), "abc");
        assert!(matches!(parse(&short), Err(ManifestError::InvalidEntry(_))));
        let not_hex = manifest_with_path("a.exe").replace(&fixture_sha256(), &"z".repeat(64));
        assert!(matches!(parse(&not_hex), Err(ManifestError::InvalidEntry(_))));
        let duplicate = format!(
            r#"{{"manifest_version":1,"version":"1","files":[{{"path":"A.exe","sha256":"{0}"}},{{"path":"a.EXE","sha256":"{0}"}}]}}"#,
            fixture_sha256()
        );
        assert!(matches!(parse(&duplicate), Err(ManifestError::DuplicatePath)));
        let none = r#"{"manifest_version":1,"version":"1","files":[]}"#;
        assert!(matches!(parse(none), Err(ManifestError::InvalidEntry(_))));
    }

    #[test]
    fn check_file_accepts_the_listed_content_and_rejects_everything_else() {
        let manifest = ReleaseManifest::verify(MANIFEST, SIGNATURE, DEV_PUBLIC_KEY)
            .unwrap_or_else(|error| panic!("fixture must verify: {error}"));
        let dir = scratch_dir("check-file");
        let name = Path::new("SensorAgent.exe");

        std::fs::write(dir.join(name), b"sidecar-fixture")
            .unwrap_or_else(|error| panic!("write: {error}"));
        assert!(manifest.check_file(&dir, name).is_ok());

        std::fs::write(dir.join(name), b"sidecar-fixture!")
            .unwrap_or_else(|error| panic!("write: {error}"));
        assert!(matches!(manifest.check_file(&dir, name), Err(ManifestError::HashMismatch)));

        assert!(matches!(
            manifest.check_file(&dir, Path::new("Other.exe")),
            Err(ManifestError::NotListed)
        ));

        std::fs::remove_file(dir.join(name)).unwrap_or_else(|error| panic!("remove: {error}"));
        assert!(matches!(manifest.check_file(&dir, name), Err(ManifestError::Io(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
