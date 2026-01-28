#![allow(clippy::multiple_crate_versions)]

use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::Command;

pub const APP_NAME: &str = "beeminder";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiKey {
    Literal(String),
    Env { env: String },
    Cmd { cmd: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_true")]
    pub show_pledge: bool,
    #[serde(default)]
    pub show_last_value: bool,
    #[serde(default = "default_datapoints_limit")]
    pub datapoints_limit: usize,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_pledge: true,
            show_last_value: false,
            datapoints_limit: default_datapoints_limit(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    #[serde(default = "default_true")]
    pub refresh_on_start: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            refresh_on_start: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeeConfig {
    pub api_key: ApiKey,
    pub default_user: Option<String>,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub tui: TuiConfig,
}

impl Default for BeeConfig {
    fn default() -> Self {
        Self {
            api_key: ApiKey::Literal(String::new()),
            default_user: None,
            display: DisplayConfig::default(),
            tui: TuiConfig::default(),
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_datapoints_limit() -> usize {
    20
}

#[derive(Debug, thiserror::Error)]
pub enum BeeConfigError {
    #[error("config error: {0}")]
    Confy(#[from] confy::ConfyError),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("missing api key in config; set `api_key` in the beeminder config file")]
    MissingApiKey,
    #[error("environment variable '{env}' not found")]
    MissingEnv { env: String },
    #[error("api key command failed: {cmd}: {message}")]
    CommandFailed { cmd: String, message: String },
    #[error("failed to execute api key command '{cmd}': {source}")]
    CommandExec { cmd: String, source: io::Error },
    #[error("api key command returned empty output: {cmd}")]
    CommandEmpty { cmd: String },
    #[error(
        "api key required but stdin is not interactive; set `api_key` in {path} (example: api_key = \"YOUR_KEY\" or api_key = {{ cmd = \"...\" }})",
        path = .path.display()
    )]
    NonInteractive { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, BeeConfigError>;

impl BeeConfig {
    /// Loads the config file from the standard OS location.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or deserialized.
    pub fn load() -> Result<Self> {
        Ok(confy::load(APP_NAME, None)?)
    }

    /// Loads config or walks the user through onboarding the API key.
    ///
    /// # Errors
    /// Returns an error if the config cannot be loaded, the API key cannot be
    /// resolved, or onboarding fails (including non-interactive stdin).
    pub fn load_or_onboard() -> Result<Self> {
        let mut config = Self::load()?;
        if let ApiKey::Literal(value) = &config.api_key {
            if !value.trim().is_empty() {
                return Ok(config);
            }
        } else {
            config.api_key.resolve()?;
            return Ok(config);
        }

        config = config.onboard_api_key()?;
        Ok(config)
    }

    /// Stores the config to the standard OS location.
    ///
    /// # Errors
    /// Returns an error if the config cannot be serialized or written.
    pub fn store(&self) -> Result<()> {
        confy::store(APP_NAME, None, self)?;
        Ok(())
    }

    /// Resolves the API key from the configured source.
    ///
    /// # Errors
    /// Returns an error if the API key cannot be resolved or is empty.
    pub fn api_key(&self) -> Result<String> {
        self.api_key.resolve()
    }

    fn onboard_api_key(mut self) -> Result<Self> {
        let config_path = confy::get_configuration_file_path(APP_NAME, None)?;
        if !io::stdin().is_terminal() {
            return Err(BeeConfigError::NonInteractive { path: config_path });
        }

        if !config_path.as_os_str().is_empty() {
            eprintln!(
                "Beeminder config not found or missing api_key. It will be stored at: {}",
                config_path.display()
            );
        }

        eprint!("Enter your Beeminder API key: ");
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(BeeConfigError::MissingApiKey);
        }

        self.api_key = ApiKey::Literal(trimmed.to_string());
        self.store()?;
        Ok(self)
    }
}

impl ApiKey {
    fn resolve(&self) -> Result<String> {
        match self {
            Self::Literal(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(BeeConfigError::MissingApiKey);
                }
                Ok(trimmed.to_string())
            }
            Self::Env { env } => {
                let value = std::env::var(env)
                    .map_err(|_| BeeConfigError::MissingEnv { env: env.clone() })?;
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(BeeConfigError::MissingApiKey);
                }
                Ok(trimmed.to_string())
            }
            Self::Cmd { cmd } => {
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .output()
                    .map_err(|e| BeeConfigError::CommandExec {
                        cmd: cmd.clone(),
                        source: e,
                    })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(BeeConfigError::CommandFailed {
                        cmd: cmd.clone(),
                        message: stderr.trim().to_string(),
                    });
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let trimmed = stdout.trim();
                if trimmed.is_empty() {
                    return Err(BeeConfigError::CommandEmpty { cmd: cmd.clone() });
                }
                Ok(trimmed.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiKey, BeeConfigError};

    #[test]
    fn resolves_literal_key() {
        let key = ApiKey::Literal(" literal ".to_string());
        assert_eq!(key.resolve().unwrap(), "literal");
    }

    #[test]
    fn resolves_env_key() {
        let var = format!("BEECONFIG_TEST_KEY_{}", std::process::id());
        std::env::set_var(&var, "envvalue");
        let key = ApiKey::Env { env: var.clone() };
        assert_eq!(key.resolve().unwrap(), "envvalue");
        std::env::remove_var(&var);
    }

    #[test]
    fn resolves_cmd_key() {
        let key = ApiKey::Cmd {
            cmd: "printf 'cmdvalue'".to_string(),
        };
        assert_eq!(key.resolve().unwrap(), "cmdvalue");
    }

    #[test]
    fn cmd_empty_output_is_error() {
        let key = ApiKey::Cmd {
            cmd: "printf ''".to_string(),
        };
        let err = key.resolve().unwrap_err();
        assert!(matches!(err, BeeConfigError::CommandEmpty { .. }));
    }
}
