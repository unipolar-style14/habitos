use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    pub ai: AiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// "ollama" | "openai-compatible" | unset
    pub backend: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    /// API key (for openai-compatible backends). Stored in config for V1
    /// convenience; OS keychain integration is deferred to V1.1.
    pub api_key: Option<String>,
    pub timeout_secs: u64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            backend: None,
            model: None,
            endpoint: None,
            api_key: None,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    file: ConfigFile,
    data_dir: PathBuf,
}

impl Config {
    pub fn load_or_init() -> Result<Self, CoreError> {
        Self::load_or_init_in(resolve_data_dir()?)
    }

    pub fn load_or_init_in(data_dir: PathBuf) -> Result<Self, CoreError> {
        std::fs::create_dir_all(&data_dir)?;
        let config_path = data_dir.join("config.toml");
        let file = if config_path.exists() {
            let text = std::fs::read_to_string(&config_path)?;
            toml::from_str(&text)?
        } else {
            let default = ConfigFile::default();
            let text = toml::to_string_pretty(&default)?;
            std::fs::write(&config_path, text)?;
            default
        };
        Ok(Self { file, data_dir })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn config_path(&self) -> PathBuf {
        self.data_dir.join("config.toml")
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("habitos.db")
    }

    pub fn ai(&self) -> &AiConfig {
        &self.file.ai
    }

    /// Replace the `[ai]` block and persist to disk.
    pub fn save_ai(&mut self, ai: AiConfig) -> Result<(), CoreError> {
        self.file.ai = ai;
        let text = toml::to_string_pretty(&self.file)?;
        std::fs::write(self.config_path(), text)?;
        Ok(())
    }
}

fn resolve_data_dir() -> Result<PathBuf, CoreError> {
    if let Ok(override_path) = std::env::var("HABITOS_HOME") {
        return Ok(PathBuf::from(override_path));
    }
    let proj_dirs = directories::ProjectDirs::from("", "", "habitos").ok_or(CoreError::NoHome)?;
    Ok(proj_dirs.data_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_default_config_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config::load_or_init_in(tmp.path().to_path_buf()).unwrap();
        assert_eq!(cfg.data_dir(), tmp.path());
        assert!(cfg.config_path().exists(), "config.toml should be created");
        assert_eq!(cfg.ai().timeout_secs, 30);
    }

    #[test]
    fn round_trips_existing_config() {
        let tmp = tempfile::tempdir().unwrap();
        let custom = ConfigFile {
            ai: AiConfig {
                backend: Some("ollama".into()),
                model: Some("gemma2:2b".into()),
                endpoint: Some("http://127.0.0.1:11434".into()),
                api_key: None,
                timeout_secs: 45,
            },
        };
        std::fs::write(
            tmp.path().join("config.toml"),
            toml::to_string_pretty(&custom).unwrap(),
        )
        .unwrap();

        let cfg = Config::load_or_init_in(tmp.path().to_path_buf()).unwrap();
        assert_eq!(cfg.ai().backend.as_deref(), Some("ollama"));
        assert_eq!(cfg.ai().timeout_secs, 45);
    }
}
