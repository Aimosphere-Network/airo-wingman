/// Configuration for the application.
#[derive(Debug)]
pub struct Config {
    /// The port to listen on. Defaults to 9090. Can be overridden with the `AW_PORT` environment
    /// variable.
    pub http_port: u16,
    /// The blockchain node to connect to. Defaults to `ws://127.0.0.1:9944`.
    /// Can be overridden with the `AIRO_NODE` environment variable.
    pub airo_node: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            http_port: envmnt::get_u16("AW_PORT", 9090),
            airo_node: envmnt::get_or("AIRO_NODE", "ws://127.0.0.1:9944"),
        }
    }
}
