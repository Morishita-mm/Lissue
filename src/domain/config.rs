use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub output: OutputConfig,
    pub integration: IntegrationConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputConfig {
    pub default_format: String, // human or json
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IntegrationConfig {
    pub git_mv_hook: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: OutputConfig {
                default_format: "human".to_string(),
            },
            integration: IntegrationConfig { git_mv_hook: true },
        }
    }
}
