use camino::Utf8PathBuf;
use config::Config;
use once_cell::unsync::OnceCell;
use anyhow::{Result, anyhow};
use std::cell::RefCell;

use crate::Cli;
use super::{default_user_config_path, write_to_disk_json, Error};

thread_local! {
    static CONFIG: OnceCell<RefCell<UserConfig>> = OnceCell::new();
}

#[derive(Debug, Clone)]
pub struct UserConfig {
    inner: UserConfigInner,
    disk_config: UserConfigInner,
    path: Utf8PathBuf,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct UserConfigInner {
    token: Option<Token>,
    default_org: Option<String>,
    #[serde(default = "default_url")]
    url: String,
    #[serde(default = "default_permissions")]
    permissions: Vec<String>,
}

fn default_url() -> String {
    "https://api.molnett.org".to_string()
}

fn default_permissions() -> Vec<String> {
    vec!["superadmin".to_string()]
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
}

impl UserConfig {
    pub fn load_from_disk() -> Result<()> {
        let config_path = default_user_config_path()?;
        let config = UserConfigLoader::load(&config_path)?;
        CONFIG.with(|cell| cell.set(RefCell::new(config)))
            .map_err(|_| anyhow!("UserConfig has already been initialized"))
    }

    pub fn apply_cli_options(cli: &Cli) {
        CONFIG.with(|cell| {
            let mut config = cell.get().expect("UserConfig has not been initialized").borrow_mut();

            if let Some(url) = &cli.url {
                config.inner.url = url.to_string();
            }

            config.inner.permissions = vec!["superadmin".to_string()];
        });
    }

    pub fn get_token() -> Option<String> {
        CONFIG.with(|cell| {
            let config = cell.get().expect("UserConfig has not been initialized").borrow();
            config.inner.token.as_ref().map(|u| u.access_token.as_str()).map(|t| t.to_string())
        })
    }

    pub fn get_url() -> String {
        CONFIG.with(|cell| {
            let config = cell.get().expect("UserConfig has not been initialized").borrow();
            config.inner.url.clone()
        })
    }

    pub fn get_default_org() -> Option<String> {
        CONFIG.with(|cell| {
            let config = cell.get().expect("UserConfig has not been initialized").borrow();
            config.inner.default_org.clone()
        })
    }

    pub fn get_permissions() -> Vec<String> {
        CONFIG.with(|cell| {
            let config = cell.get().expect("UserConfig has not been initialized").borrow();
            config.inner.permissions.clone()
        })
    }

    pub fn set_token(token: Token) -> Result<(), Error> {
        CONFIG.with(|cell| {
            let mut config = cell.get().expect("UserConfig has not been initialized").borrow_mut();
            config.disk_config.token = Some(token.clone());
            config.inner.token = Some(token);
            write_to_disk_json(&config.path, &config.disk_config)
        })
    }

    pub fn set_default_org(org_name: String) -> Result<(), Error> {
        CONFIG.with(|cell| {
            let mut config = cell.get().expect("UserConfig has not been initialized").borrow_mut();
            config.disk_config.default_org = Some(org_name.clone());
            config.inner.default_org = Some(org_name);
            write_to_disk_json(&config.path, &config.disk_config)
        })
    }

    pub fn is_token_expired() -> bool {
        CONFIG.with(|cell| {
            let config = cell.get().expect("UserConfig has not been initialized").borrow();
            config.inner.token
              .as_ref()
              .map(|u| u.expiry.is_some() && u.expiry.unwrap() < chrono::Utc::now())
              .unwrap_or(false)
        })
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
            inner: config.try_deserialize()?,
            disk_config: disk_config.try_deserialize()?,
            path: path.clone(),
        })
    }
}
