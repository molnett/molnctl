use camino::Utf8PathBuf;
use config::Config;

use crate::Cli;

use super::{default_user_config_path, write_to_disk_json, Error};

#[derive(Debug, Clone)]
pub struct UserConfig {
    config: UserConfigInner,
    disk_config: UserConfigInner,
    path: Utf8PathBuf,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct UserConfigInner {
    token: Option<Token>,
    #[serde(default = "default_host")]
    host: String,
}

fn default_host() -> String {
    "api.molnett.org".to_string()
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expiry: Option<chrono::DateTime<chrono::Utc>>,
}

impl Token {
    pub fn new() -> Self {
        Token {
            access_token: "".to_string(),
            refresh_token: None,
            expiry: None,
        }
    }
}

pub struct UserConfigLoader {
    pub path: Utf8PathBuf,
}

impl UserConfig {
    pub fn new(cli: &Cli) -> Self {
        let config_path = match &cli.config {
            Some(path) => path.clone(),
            None => default_user_config_path().expect("No config path provided and default path not found"),
        };
        let mut config = UserConfigLoader::load(&config_path).expect("Loading config from disk failed");

        if let Some(h) = &cli.host {
            config.set_host(h.to_string());
        }

        config
    }
    pub fn get_token(&self) -> Option<&str> {
        self.config.token.as_ref().map(|u| u.access_token.as_str())
    }
    pub fn write_token(&mut self, token: Token) -> Result<(), super::Error> {
        self.disk_config.token = Some(token.clone());
        self.config.token = Some(token);

        write_to_disk_json(&self.path, &self.disk_config)
    }
    pub fn get_host(&self) -> &str {
        self.config.host.as_ref()
    }
    fn set_host(&mut self, host: String) {
        self.config.host = host;
    }
}

impl UserConfigLoader {
    pub fn load(path: &Utf8PathBuf) -> Result<UserConfig, Error> {
        let disk_config = Config::builder()
            .add_source(
                config::File::with_name(path.as_str())
                    .format(config::FileFormat::Json)
                    .required(false),
            )
            .build()?;

        let config = Config::builder().add_source(disk_config.clone()).build()?;

        Ok(UserConfig {
            config: config.try_deserialize()?,
            disk_config: disk_config.try_deserialize()?,
            path: path.clone(),
        })
    }
}
