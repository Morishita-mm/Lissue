use crate::domain::config::Config;
use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;

pub struct YamlConfigRepository {
    path: std::path::PathBuf,
}

impl YamlConfigRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        let file = File::create(&self.path)
            .with_context(|| format!("Failed to create config file: {:?}", self.path))?;
        serde_yaml::to_writer(file, config).with_context(|| "Failed to write config to YAML")?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load(&self) -> Result<Config> {
        if !self.path.exists() {
            return Ok(Config::default());
        }
        let file = File::open(&self.path)
            .with_context(|| format!("Failed to open config file: {:?}", self.path))?;
        let config =
            serde_yaml::from_reader(file).with_context(|| "Failed to parse config from YAML")?;
        Ok(config)
    }
}
