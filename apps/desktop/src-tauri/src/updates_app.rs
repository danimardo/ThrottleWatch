//! Connects [`UpdateService`](crate::updates::UpdateService) to Tauri: the official updater plugin
//! as the transport (one fixed endpoint, one trusted key), the four commands of
//! `contracts/application-commands.md`, the 24 h scheduler and the operations that block an
//! install. Nothing here accepts a URL, a path, a key or a channel from the interface.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::commands::CommandError;
use crate::release_manifest::trusted_public_key;
use crate::storage::AppState;
use crate::updater::UpdateMachine;
use crate::updates::{
    Blocker, ENDPOINT, ReleaseInfo, TransportError, UpdateFailure, UpdateService, UpdateStateDto,
    UpdateTransport,
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// First automatic look after startup, and how often the cadence is re-evaluated afterwards.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(45);
const SCHEDULER_TICK: Duration = Duration::from_secs(30 * 60);

/// Standard base64 (RFC 4648, padded): the updater plugin wants its public key as the base64 of
/// the minisign key file.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = chunk
            .iter()
            .enumerate()
            .fold(0_u32, |acc, (i, byte)| acc | u32::from(*byte) << (16 - 8 * i));
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(char::from(ALPHABET[((value >> (18 - 6 * index)) & 0x3f) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The public key the updater plugin must verify against, or empty when this build trusts none:
/// an empty key makes every signature check fail, so nothing can be verified or installed.
pub fn plugin_public_key() -> String {
    trusted_public_key().map(|key| base64(key.as_bytes())).unwrap_or_default()
}

fn classify(error: &tauri_plugin_updater::Error) -> TransportError {
    use tauri_plugin_updater::Error;
    match error {
        Error::Minisign(_) | Error::Base64(_) | Error::SignatureUtf8(_) => {
            TransportError::SignatureInvalid
        }
        _ => TransportError::Network,
    }
}

/// The official plugin against one endpoint, with the artifact held in memory between the
/// verified download and the install.
pub struct PluginTransport<R: Runtime> {
    app: AppHandle<R>,
    endpoint: tauri::Url,
    public_key: String,
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
    before_install: Box<dyn Fn() + Send>,
}

impl<R: Runtime> PluginTransport<R> {
    /// The endpoint and key of this build: the fixed GitHub Releases endpoint and the trusted key
    /// (none in a release build until the real key exists).
    pub fn production(app: AppHandle<R>, before_install: Box<dyn Fn() + Send>) -> Option<Self> {
        Some(Self::with(
            app,
            tauri::Url::parse(ENDPOINT).ok()?,
            plugin_public_key(),
            before_install,
        ))
    }

    /// Only the production constructor and the tests choose an endpoint; no command does.
    pub(crate) fn with(
        app: AppHandle<R>,
        endpoint: tauri::Url,
        public_key: String,
        before_install: Box<dyn Fn() + Send>,
    ) -> Self {
        Self { app, endpoint, public_key, update: None, bytes: None, before_install }
    }
}

impl<R: Runtime> UpdateTransport for PluginTransport<R> {
    fn check(&mut self) -> Result<Option<ReleaseInfo>, TransportError> {
        if self.public_key.is_empty() {
            return Err(TransportError::NoTrustedKey);
        }
        let updater = self
            .app
            .updater_builder()
            .pubkey(self.public_key.clone())
            .endpoints(vec![self.endpoint.clone()])
            .map_err(|_| TransportError::Network)?
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_| TransportError::Network)?;
        let found =
            tauri::async_runtime::block_on(updater.check()).map_err(|error| classify(&error))?;
        Ok(found.map(|update| {
            let release = ReleaseInfo {
                version: update.version.clone(),
                notes: update.body.clone().unwrap_or_default(),
            };
            self.update = Some(update);
            release
        }))
    }

    fn download(
        &mut self,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<u64, TransportError> {
        let Some(update) = self.update.as_ref() else { return Err(TransportError::Network) };
        let mut downloaded = 0_u64;
        let bytes = tauri::async_runtime::block_on(update.download(
            |chunk, total| {
                downloaded += chunk as u64;
                progress(downloaded, total);
            },
            || {},
        ))
        .map_err(|error| classify(&error))?;
        let length = bytes.len() as u64;
        self.bytes = Some(bytes);
        Ok(length)
    }

    fn install(&mut self) -> Result<(), TransportError> {
        let (Some(update), Some(bytes)) = (self.update.as_ref(), self.bytes.as_ref()) else {
            return Err(TransportError::Install);
        };
        (self.before_install)();
        update.install(bytes).map_err(|_| TransportError::Install)
    }

    fn discard(&mut self) {
        self.update = None;
        self.bytes = None;
    }
}

/// The service and the last state the interface was told about (readable while a download holds
/// the service).
pub struct UpdateHandle {
    service: Mutex<UpdateService>,
    snapshot: Mutex<UpdateStateDto>,
}

impl UpdateHandle {
    pub fn new(service: UpdateService) -> Self {
        let snapshot = service.dto();
        Self { service: Mutex::new(service), snapshot: Mutex::new(snapshot) }
    }

    pub(crate) fn snapshot(&self) -> UpdateStateDto {
        self.snapshot
            .lock()
            .map(|dto| dto.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }

    fn remember(&self, dto: &UpdateStateDto) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            *snapshot = dto.clone();
        }
    }

    /// The `updates.enabled` preference changed.
    pub fn set_enabled(&self, app: &AppHandle, enabled: bool) {
        let Ok(mut service) = self.service.lock() else { return };
        service.set_enabled(enabled);
        let dto = service.dto();
        persist(app, &service);
        drop(service);
        publish(app, self, &dto);
    }
}

/// Export, import and wipe register here so an install waits for them.
#[derive(Default, Clone)]
pub struct BusyOperations(Arc<Mutex<Vec<Blocker>>>);

/// Ends the registration when dropped; owns its share of the list so it outlives the borrow of
/// the managed state it was created from.
pub struct BusyGuard {
    operations: Arc<Mutex<Vec<Blocker>>>,
    blocker: Blocker,
}

impl BusyOperations {
    pub fn begin(&self, blocker: Blocker) -> BusyGuard {
        if let Ok(mut active) = self.0.lock() {
            active.push(blocker);
        }
        BusyGuard { operations: Arc::clone(&self.0), blocker }
    }

    pub(crate) fn first(&self) -> Option<Blocker> {
        self.0.lock().ok().and_then(|active| active.first().copied())
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = self.operations.lock()
            && let Some(position) = active.iter().position(|item| *item == self.blocker)
        {
            active.remove(position);
        }
    }
}

fn install_blocker(app: &AppHandle) -> Option<Blocker> {
    if crate::commands::guided_in_progress(app) {
        return Some(Blocker::GuidedTest);
    }
    app.try_state::<BusyOperations>().and_then(|busy| busy.first())
}

fn persist(app: &AppHandle, service: &UpdateService) {
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(storage) = state.storage.lock()
    {
        let _ = storage.set_updater_state(service.machine());
    }
}

fn publish(app: &AppHandle, handle: &UpdateHandle, dto: &UpdateStateDto) {
    handle.remember(dto);
    let _ = app.emit("update:state-changed", dto);
    if dto.state == "downloading" {
        let _ = app.emit(
            "update:progress",
            serde_json::json!({ "downloaded": dto.downloaded, "total": dto.total }),
        );
    }
}

fn announce_failure(app: &AppHandle, failure: UpdateFailure, recoverable: bool) {
    let _ = app.emit(
        "update:error",
        serde_json::json!({
            "message_key": failure.code(),
            "code": failure.code(),
            "recoverable": recoverable
        }),
    );
}

fn command_error(failure: UpdateFailure) -> CommandError {
    let key = match failure {
        UpdateFailure::Blocked(Blocker::GuidedTest) => "update.blocked.guided_test",
        UpdateFailure::Blocked(Blocker::Export) => "update.blocked.export",
        UpdateFailure::Blocked(Blocker::Import) => "update.blocked.import",
        UpdateFailure::Blocked(Blocker::DataOperation) => "update.blocked.data_operation",
        other => other.code(),
    };
    CommandError { code: failure.code(), message_key: key }
}

/// One check, off the interface's thread. `automatic` checks also raise the native notification.
fn run_check(app: &AppHandle, manual: bool) {
    let Some(handle) = app.try_state::<UpdateHandle>() else { return };
    let Ok(mut service) = handle.service.try_lock() else { return };
    let now = jiff::Timestamp::now().to_string();
    let result = service.check(&now, manual, &mut |dto| publish(app, &handle, dto));
    persist(app, &service);
    drop(service);
    match result {
        Ok(Some(release)) => {
            let _ = app.emit(
                "update:available",
                serde_json::json!({ "version": release.version, "notes": release.notes }),
            );
            if !manual {
                notify_available(app, &release.version);
            }
        }
        Ok(None) => {}
        Err(failure) => announce_failure(
            app,
            failure,
            !matches!(
                failure,
                UpdateFailure::Transport(
                    TransportError::SignatureInvalid | TransportError::NoTrustedKey
                )
            ),
        ),
    }
}

fn notify_available(app: &AppHandle, version: &str) {
    let locale = crate::tray::current_locale(app);
    let title = crate::i18n::text(locale, "native.notification.update_available.title");
    let body = crate::i18n::text(locale, "native.notification.update_available.body")
        .replace("{version}", version);
    crate::alerting::notify(app, &title, &body);
}

fn run_download(app: &AppHandle) {
    let Some(handle) = app.try_state::<UpdateHandle>() else { return };
    let Ok(mut service) = handle.service.try_lock() else { return };
    let result = service.download(&mut |dto| publish(app, &handle, dto));
    persist(app, &service);
    drop(service);
    if let Err(failure) = result {
        announce_failure(
            app,
            failure,
            !matches!(failure, UpdateFailure::Transport(TransportError::SignatureInvalid)),
        );
    }
}

fn run_install(app: &AppHandle) {
    let Some(handle) = app.try_state::<UpdateHandle>() else { return };
    let Ok(mut service) = handle.service.try_lock() else { return };
    let blocker = install_blocker(app);
    let result = service.install(blocker, &mut |dto| publish(app, &handle, dto));
    persist(app, &service);
    drop(service);
    if let Err(failure) = result {
        announce_failure(app, failure, true);
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckRequest {
    pub manual: bool,
}

#[tauri::command]
pub fn get_update_state(handle: State<'_, UpdateHandle>) -> UpdateStateDto {
    handle.snapshot()
}

#[tauri::command]
pub fn check_for_update(
    app: AppHandle,
    handle: State<'_, UpdateHandle>,
    request: CheckRequest,
) -> Result<UpdateStateDto, CommandError> {
    let snapshot = handle.snapshot();
    if !snapshot.enabled {
        return Err(command_error(UpdateFailure::Machine(crate::updater::UpdateError::Disabled)));
    }
    std::thread::Builder::new()
        .name("update-check".to_owned())
        .spawn(move || run_check(&app, request.manual))
        .map_err(|_| CommandError { code: "update.busy", message_key: "update.busy" })?;
    Ok(snapshot)
}

#[tauri::command]
pub fn download_update(
    app: AppHandle,
    handle: State<'_, UpdateHandle>,
) -> Result<UpdateStateDto, CommandError> {
    let snapshot = handle.snapshot();
    if snapshot.state != "available" {
        return Err(command_error(UpdateFailure::Machine(
            crate::updater::UpdateError::NoAvailableUpdate,
        )));
    }
    std::thread::Builder::new()
        .name("update-download".to_owned())
        .spawn(move || run_download(&app))
        .map_err(|_| CommandError { code: "update.busy", message_key: "update.busy" })?;
    Ok(snapshot)
}

#[tauri::command]
pub fn install_update(
    app: AppHandle,
    handle: State<'_, UpdateHandle>,
) -> Result<UpdateStateDto, CommandError> {
    let snapshot = handle.snapshot();
    if snapshot.state != "verified" {
        return Err(command_error(UpdateFailure::Machine(
            crate::updater::UpdateError::NotVerified,
        )));
    }
    if let Some(blocker) = install_blocker(&app) {
        return Err(command_error(UpdateFailure::Blocked(blocker)));
    }
    std::thread::Builder::new()
        .name("update-install".to_owned())
        .spawn(move || run_install(&app))
        .map_err(|_| CommandError { code: "update.busy", message_key: "update.busy" })?;
    Ok(snapshot)
}

/// The persisted machine, with the switch taken from the preference and nothing in flight.
pub fn restore_machine(stored: UpdateMachine, enabled: bool) -> UpdateMachine {
    let mut machine = stored;
    machine.recover();
    machine.set_enabled(enabled);
    machine
}

/// Looks at the cadence for the life of the process. With the updater off it does nothing at all:
/// no request, no timer that touches the network.
pub fn spawn_scheduler(app: AppHandle) {
    let _ = std::thread::Builder::new().name("update-scheduler".to_owned()).spawn(move || {
        std::thread::sleep(FIRST_CHECK_DELAY);
        loop {
            let due = app.try_state::<UpdateHandle>().is_some_and(|handle| {
                handle.service.try_lock().is_ok_and(|service| {
                    service.automatic_check_due(&jiff::Timestamp::now().to_string())
                })
            });
            if due {
                run_check(&app, false);
            }
            std::thread::sleep(SCHEDULER_TICK);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/updater");

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!("{FIXTURES}/{name}"))
            .unwrap_or_else(|error| panic!("fixture {name}: {error}"))
    }

    #[test]
    fn base64_matches_the_standard_vectors() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected, "{input:?}");
        }
    }

    #[test]
    fn a_release_build_trusts_no_key_yet_and_a_debug_build_trusts_the_development_one() {
        let key = plugin_public_key();
        if cfg!(any(debug_assertions, feature = "e2e")) {
            assert!(key.starts_with("dW50cnVzdGVk"), "base64 of «untrusted comment»: {key}");
        } else {
            assert!(key.is_empty());
        }
    }

    /// A local release server: the manifest, the good artifact and a tampered one. Counts every
    /// request so «zero traffic» is an observation, not a hope.
    struct Server {
        port: u16,
        hits: Arc<AtomicUsize>,
        stop: Arc<AtomicBool>,
    }

    impl Server {
        fn start() -> Self {
            let listener =
                TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| panic!("bind: {error}"));
            let port = listener.local_addr().map(|address| address.port()).unwrap_or(0);
            let hits = Arc::new(AtomicUsize::new(0));
            let stop = Arc::new(AtomicBool::new(false));
            let (thread_hits, thread_stop) = (Arc::clone(&hits), Arc::clone(&stop));
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if thread_stop.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Ok(stream) = stream {
                        thread_hits.fetch_add(1, Ordering::SeqCst);
                        Self::serve(stream, port);
                    }
                }
            });
            Self { port, hits, stop }
        }

        fn serve(mut stream: TcpStream, port: u16) {
            let mut buffer = [0_u8; 2048];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]);
            let path = request.split_whitespace().nth(1).unwrap_or("/");
            let signature = base64(&fixture("installer.bin.minisig"));
            let manifest = |artifact: &str| {
                serde_json::json!({
                    "version": "9.9.9",
                    "notes": "Fixture release",
                    "pub_date": "2026-09-21T08:00:00Z",
                    "platforms": { "windows-x86_64": {
                        "signature": signature,
                        "url": format!("http://127.0.0.1:{port}/{artifact}")
                    }}
                })
                .to_string()
                .into_bytes()
            };
            let (status, body): (&str, Vec<u8>) = match path {
                "/manifest.json" => ("200 OK", manifest("installer.bin")),
                "/manifest-tampered.json" => ("200 OK", manifest("installer-tampered.bin")),
                "/installer.bin" => ("200 OK", fixture("installer.bin")),
                "/installer-tampered.bin" => ("200 OK", fixture("installer-tampered.bin")),
                _ => ("404 Not Found", Vec::new()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }

        fn url(&self, path: &str) -> tauri::Url {
            tauri::Url::parse(&format!("http://127.0.0.1:{}/{path}", self.port))
                .unwrap_or_else(|error| panic!("url: {error}"))
        }

        fn hits(&self) -> usize {
            self.hits.load(Ordering::SeqCst)
        }
    }

    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
        }
    }

    fn app() -> tauri::App<MockRuntime> {
        let mut context = mock_context(noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".to_owned(),
            serde_json::json!({
                "pubkey": "",
                "endpoints": [],
                "dangerousInsecureTransportProtocol": true
            }),
        );
        mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap_or_else(|error| panic!("mock app: {error}"))
    }

    fn service(
        app: &tauri::App<MockRuntime>,
        server: &Server,
        manifest: &str,
        enabled: bool,
        key: String,
    ) -> UpdateService {
        let transport =
            PluginTransport::with(app.handle().clone(), server.url(manifest), key, Box::new(|| {}));
        UpdateService::new(UpdateMachine::new(enabled), Box::new(transport), "0.0.0")
    }

    fn test_key() -> String {
        base64(&fixture("test.pub"))
    }

    const NOW: &str = "2026-09-21T08:00:00Z";

    #[test]
    fn the_real_plugin_finds_downloads_and_verifies_a_signed_release() {
        let (app, server) = (app(), Server::start());
        let mut service = service(&app, &server, "manifest.json", true, test_key());

        let release = service
            .check(NOW, true, &mut |_| {})
            .unwrap_or_else(|error| panic!("check: {error:?}"))
            .unwrap_or_else(|| panic!("a newer release is expected"));
        assert_eq!(
            (release.version.as_str(), release.notes.as_str()),
            ("9.9.9", "Fixture release")
        );
        assert_eq!(service.dto().state, "available");

        let mut chunks = Vec::new();
        service
            .download(&mut |dto| chunks.push(dto.state))
            .unwrap_or_else(|error| panic!("download: {error:?}"));

        assert_eq!(service.dto().state, "verified");
        assert!(chunks.contains(&"downloading"));
        assert_eq!(server.hits(), 2, "one manifest request and one artifact request");
    }

    #[test]
    fn an_artifact_that_does_not_match_its_signature_is_rejected_and_never_reaches_verified() {
        let (app, server) = (app(), Server::start());
        let mut service = service(&app, &server, "manifest-tampered.json", true, test_key());
        service.check(NOW, true, &mut |_| {}).ok();

        let result = service.download(&mut |_| {});

        assert_eq!(result, Err(UpdateFailure::Transport(TransportError::SignatureInvalid)));
        let dto = service.dto();
        assert_eq!(dto.state, "error");
        assert_eq!(dto.error_code.as_deref(), Some("update.signature_invalid"));
        assert_eq!(dto.recoverable, Some(false));
        assert_eq!(
            service.install(None, &mut |_| {}),
            Err(UpdateFailure::Machine(crate::updater::UpdateError::NotVerified))
        );
    }

    #[test]
    fn a_key_that_did_not_sign_the_release_rejects_it() {
        let (app, server) = (app(), Server::start());
        let other = base64(crate::release_manifest::DEV_PUBLIC_KEY.as_bytes());
        let mut service = service(&app, &server, "manifest.json", true, other);
        service.check(NOW, true, &mut |_| {}).ok();
        assert_eq!(
            service.download(&mut |_| {}),
            Err(UpdateFailure::Transport(TransportError::SignatureInvalid))
        );
    }

    #[test]
    fn a_build_without_a_trusted_key_cannot_even_check() {
        let (app, server) = (app(), Server::start());
        let mut service = service(&app, &server, "manifest.json", true, String::new());
        assert_eq!(
            service.check(NOW, true, &mut |_| {}),
            Err(UpdateFailure::Transport(TransportError::NoTrustedKey))
        );
        assert_eq!(server.hits(), 0);
    }

    #[test]
    fn with_the_updater_off_the_server_sees_no_request_at_all() {
        let (app, server) = (app(), Server::start());
        let mut service = service(&app, &server, "manifest.json", false, test_key());
        for manual in [false, true] {
            let _ = service.check(NOW, manual, &mut |_| {});
        }
        let _ = service.download(&mut |_| {});
        let _ = service.install(None, &mut |_| {});
        assert_eq!(server.hits(), 0, "gate 12: zero traffic while the updater is off");
    }

    #[test]
    fn an_unreachable_endpoint_is_a_recoverable_error_not_a_crash() {
        let app = app();
        let transport = PluginTransport::with(
            app.handle().clone(),
            tauri::Url::parse("http://127.0.0.1:9/manifest.json")
                .unwrap_or_else(|error| panic!("url: {error}")),
            test_key(),
            Box::new(|| {}),
        );
        let mut service =
            UpdateService::new(UpdateMachine::new(true), Box::new(transport), "0.0.0");
        assert_eq!(
            service.check(NOW, true, &mut |_| {}),
            Err(UpdateFailure::Transport(TransportError::Network))
        );
        assert_eq!(service.dto().recoverable, Some(true));
    }

    #[test]
    fn busy_operations_are_tracked_until_they_end() {
        let busy = BusyOperations::default();
        assert_eq!(busy.first(), None);
        {
            let _export = busy.begin(Blocker::Export);
            let _import = busy.begin(Blocker::Import);
            assert_eq!(busy.first(), Some(Blocker::Export));
        }
        assert_eq!(busy.first(), None);
    }

    #[test]
    fn a_restored_machine_follows_the_preference_and_starts_idle() {
        let mut stored = UpdateMachine::new(true);
        stored.begin_check(NOW, true).ok();
        let restored = restore_machine(stored, false);
        assert!(!restored.enabled());
        assert_eq!(restored.state(), &crate::updater::UpdateState::Idle);
        assert_eq!(restored.last_check(), Some(NOW));
    }
}
