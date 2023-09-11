use camino::Utf8PathBuf;
use config::Config;

use super::{write_to_disk, Error};

#[derive(Debug)]
pub struct UserConfig {
    pub config: UserConfigInner,
    pub disk_config: UserConfigInner,
    pub path: Utf8PathBuf,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct UserConfigInner {
    pub token: Option<Token>,
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
    pub fn token(&self) -> Option<&str> {
        self.config.token.as_ref().map(|u| u.access_token.as_str())
    }
    pub fn set_token(&mut self, token: Token) -> Result<(), super::Error> {
        self.disk_config.token = Some(token.clone());
        self.config.token = Some(token);

        write_to_disk(&self.path, &self.disk_config)
    }
}

impl UserConfigLoader {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(self) -> Result<UserConfig, Error> {
        let disk_config = Config::builder()
            .add_source(
                config::File::with_name(self.path.as_str())
                    .format(config::FileFormat::Json)
                    .required(false),
            )
            .build()?;

        let config = Config::builder().add_source(disk_config.clone()).build()?;

        Ok(UserConfig {
            config: config.try_deserialize()?,
            disk_config: disk_config.try_deserialize()?,
            path: self.path,
        })
    }
}
