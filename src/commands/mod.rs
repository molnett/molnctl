use anyhow::Result;
use once_cell::sync::OnceCell;

use crate::{
    api::APIClient,
    config::{
        application::{ApplicationConfig, ApplicationConfigLoader},
        default_app_config_path, default_user_config_path,
        user::{UserConfig, UserConfigLoader},
        Error,
    },
};

pub mod auth;
pub mod initialize;
pub mod orgs;

pub struct CommandBase {
    user_config: OnceCell<UserConfig>,
    app_config: OnceCell<ApplicationConfig>,
}

impl CommandBase {
    pub fn new() -> Self {
        Self {
            user_config: OnceCell::new(),
            app_config: OnceCell::new(),
        }
    }

    pub fn api_client(&self) -> Result<APIClient> {
        let host = self.user_config()?.get_host();
        Ok(APIClient::new(host))
    }

    fn user_config_init(&self) -> Result<UserConfig, Error> {
        UserConfigLoader::load(&default_user_config_path()?)
    }

    pub fn user_config(&self) -> Result<&UserConfig, Error> {
        self.user_config.get_or_try_init(|| self.user_config_init())
    }

    pub fn user_config_mut(&mut self) -> Result<&mut UserConfig, Error> {
        self.user_config()?;
        Ok(self.user_config.get_mut().ok_or(Error::UserConfigNotInit)?)
    }

    fn app_config_init(&self) -> Result<ApplicationConfig, Error> {
        ApplicationConfigLoader::new(default_app_config_path()?).load()
    }

    pub fn app_config(&self) -> Result<&ApplicationConfig, Error> {
        self.app_config.get_or_try_init(|| self.app_config_init())
    }

    pub fn app_config_mut(&mut self) -> Result<&mut ApplicationConfig, Error> {
        self.app_config()?;
        Ok(self.app_config.get_mut().ok_or(Error::UserConfigNotInit)?)
    }
}
