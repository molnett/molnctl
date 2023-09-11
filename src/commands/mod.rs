use anyhow::Result;
use once_cell::sync::OnceCell;

use crate::{
    api::APIClient,
    config::{
        config::{UserConfig, UserConfigLoader},
        default_user_config_path, Error,
    },
};

pub mod auth;
pub mod initialize;
pub mod orgs;

pub struct CommandBase {
    user_config: OnceCell<UserConfig>,
}

impl CommandBase {
    pub fn new() -> Self {
        Self {
            user_config: OnceCell::new(),
        }
    }

    pub fn api_client(&self) -> Result<APIClient> {
        Ok(APIClient::new("http://localhost:8000"))
    }

    fn user_config_init(&self) -> Result<UserConfig, Error> {
        UserConfigLoader::new(default_user_config_path()?).load()
    }

    pub fn user_config(&self) -> Result<&UserConfig, Error> {
        self.user_config.get_or_try_init(|| self.user_config_init())
    }

    pub fn user_config_mut(&mut self) -> Result<&mut UserConfig, Error> {
        self.user_config()?;
        Ok(self.user_config.get_mut().ok_or(Error::UserConfigNotInit)?)
    }
}
