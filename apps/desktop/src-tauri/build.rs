// Build scripts talk to cargo through stdout; the crate-wide clippy denial
// targets application code, not this file (plan.md, scope clarification A2).
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn main() {
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .unwrap_or_else(|error| panic!("tauri-build failed: {error}"));
    embed_windows_manifest();
}

/// The application manifest tauri-build would embed by default (Common
/// Controls v6, required by the dialog APIs).
const APP_MANIFEST: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
    </dependentAssembly>
  </dependency>
</assembly>
"#;

/// tauri-build embeds the Windows manifest through the crate's resource
/// file, which reaches binaries but not test executables. Tests that drive the
/// Tauri runtime (`tests/acl_surface.rs`) import `TaskDialogIndirect` and abort
/// at load with `STATUS_ENTRYPOINT_NOT_FOUND` without Common Controls v6, so
/// the manifest is embedded by the MSVC linker into every linked target instead.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn embed_windows_manifest() {
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    let Some(out_dir) = std::env::var_os("OUT_DIR") else {
        return;
    };
    if !msvc {
        return;
    }
    let manifest = std::path::Path::new(&out_dir).join("app-manifest.xml");
    if std::fs::write(&manifest, APP_MANIFEST).is_ok() {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
}
