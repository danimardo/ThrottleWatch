#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevConfig {
    pub log_level: Option<&'static str>,
}

impl DevConfig {
    #[cfg(any(debug_assertions, feature = "e2e"))]
    pub const fn load() -> Self {
        Self { log_level: option_env!("TW_DEV_LOG_LEVEL") }
    }

    #[cfg(not(any(debug_assertions, feature = "e2e")))]
    pub const fn load() -> Self {
        Self { log_level: None }
    }
}
