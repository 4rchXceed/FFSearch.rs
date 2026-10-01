pub static DEFAULT_LISTEN_ADDR: &str = "localhost:8080";

pub struct McpServerConfig {
    pub listen_addr: String,
}

impl McpServerConfig {
    pub fn new() -> Self {
        return Self {
            listen_addr: String::from(DEFAULT_LISTEN_ADDR),
        };
    }
}
