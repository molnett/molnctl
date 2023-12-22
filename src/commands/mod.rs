use anyhow::Result;
use once_cell::sync::OnceCell;

use crate::{
    api::APIClient,
    config::{
        application::{ApplicationConfig, ApplicationConfigLoader},
        default_app_config_path,
        user::UserConfig,
        Error,
    },
};

pub mod auth;
pub mod initialize;
pub mod orgs;

pub struct CommandBase<'a> {
    user_config: &'a mut UserConfig,
    app_config: OnceCell<ApplicationConfig>,
}

impl CommandBase<'_> {
    pub fn new(user_config: &mut UserConfig) -> CommandBase {
        CommandBase {
            user_config,
            app_config: OnceCell::new(),
        }
    }

    pub fn api_client(&self) -> Result<APIClient> {
        let url = self.user_config.get_url();
        Ok(APIClient::new(url))
    }

    pub fn user_config(&self) -> &UserConfig {
        self.user_config
    }

    pub fn user_config_mut(&mut self) -> &mut UserConfig {
        self.user_config
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
