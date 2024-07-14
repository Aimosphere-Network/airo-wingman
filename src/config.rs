use std::{env, ffi::OsStr, fmt::Display};

/// Configuration for the application.
#[derive(Debug)]
pub struct Config {
    /// The port to listen on. Defaults to 9090. Can be overridden with the `AW_PORT` environment
    /// variable.
    pub http_port: u16,
    /// The blockchain node to connect to. Defaults to `ws://127.0.0.1:9944`.
    /// Can be overridden with the `AIRO_NODE` environment variable.
    pub airo_node: String,
    /// The secret uri of an Infrastructure Provider used for automation. Is specified in the
    /// `AIRO_SURI` environment variable. The variable is required.
    pub airo_suri: String,
}

impl Config {
    pub fn new() -> Self {
        fn get_or_panic<K: AsRef<OsStr> + Display>(key: K) -> String {
            env::var(key.as_ref())
                .unwrap_or_else(|_| panic!("🚨 Environment variable {key} is not set"))
        }

        Self {
            http_port: envmnt::get_u16("AW_PORT", 9090),
            airo_node: envmnt::get_or("AIRO_NODE", "ws://127.0.0.1:9944"),
            airo_suri: get_or_panic("AIRO_SURI"),
        }
    }
}
