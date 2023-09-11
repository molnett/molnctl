use ::config::ConfigError;
use camino::{Utf8Path, Utf8PathBuf};
use dirs_next::config_dir;
use thiserror::Error;

pub mod config;

#[derive(Error, Debug)]
pub enum Error {
    #[error("default config path not found")]
    NoDefaultConfigPath,
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Camino(#[from] camino::FromPathBufError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("user config not initialized")]
    UserConfigNotInit,
}

pub fn default_user_config_path() -> Result<Utf8PathBuf, Error> {
    Ok(Utf8PathBuf::try_from(
        config_dir()
            .map(|path| path.join("molnett").join("config.json"))
            .ok_or(Error::NoDefaultConfigPath)?,
    )?)
}

pub fn write_to_disk<T>(path: &Utf8Path, config: T) -> Result<(), Error>
where
    T: serde::Serialize,
{
    if let Some(parent_dir) = path.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }

    let config_file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(&config_file, &config)?;

    config_file.sync_all()?;

    Ok(())
}
