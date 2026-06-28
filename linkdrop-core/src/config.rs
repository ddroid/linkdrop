use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const DEFAULT_DATA_DIR: &str = "/data";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub url: Option<String>,
    pub token: Option<String>,
    pub data_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: None,
            token: None,
            data_dir: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut config = Self::default();

        if let Some(path) = config_file_path() {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(file_config) = toml::from_str::<Config>(&contents) {
                        config.merge(file_config);
                    }
                }
            }
        }

        if let Ok(url) = std::env::var("LINKDROP_URL") {
            config.url = Some(url);
        }
        if let Ok(token) = std::env::var("LINKDROP_TOKEN") {
            config.token = Some(token);
        }
        if let Ok(data_dir) = std::env::var("LINKDROP_DATA_DIR") {
            config.data_dir = Some(PathBuf::from(data_dir));
        }

        config
    }

    pub fn merge(&mut self, other: Config) {
        if other.url.is_some() {
            self.url = other.url;
        }
        if other.token.is_some() {
            self.token = other.token;
        }
        if other.data_dir.is_some() {
            self.data_dir = other.data_dir;
        }
    }

    pub fn require_url(&self) -> anyhow::Result<String> {
        self.url
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("LINKDROP_URL is not set"))
    }

    pub fn require_token(&self) -> anyhow::Result<String> {
        self.token
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("LINKDROP_TOKEN is not set"))
    }
}

pub fn config_file_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("LINKDROP_CONFIG") {
        return Some(PathBuf::from(path));
    }

    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".config/linkdrop/config.toml"));
    }

    None
}

pub fn default_data_dir() -> PathBuf {
    std::env::var("LINKDROP_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATA_DIR))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn default_data_dir_should_use_stable_absolute_path_when_env_is_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = std::env::var_os("LINKDROP_DATA_DIR");
        std::env::remove_var("LINKDROP_DATA_DIR");

        assert_eq!(default_data_dir(), PathBuf::from(DEFAULT_DATA_DIR));

        restore_data_dir_env(previous);
    }

    #[test]
    fn default_data_dir_should_allow_env_override() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = std::env::var_os("LINKDROP_DATA_DIR");
        std::env::set_var("LINKDROP_DATA_DIR", "/tmp/linkdrop-test-data");

        assert_eq!(default_data_dir(), PathBuf::from("/tmp/linkdrop-test-data"));

        restore_data_dir_env(previous);
    }

    fn restore_data_dir_env(previous: Option<std::ffi::OsString>) {
        match previous {
            Some(value) => std::env::set_var("LINKDROP_DATA_DIR", value),
            None => std::env::remove_var("LINKDROP_DATA_DIR"),
        }
    }
}
