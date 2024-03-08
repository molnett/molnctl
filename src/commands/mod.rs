use anyhow::{anyhow, Result};
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
pub mod environments;
pub mod orgs;
pub mod secrets;
pub mod services;

pub struct CommandBase<'a> {
    user_config: &'a mut UserConfig,
    app_config: OnceCell<ApplicationConfig>,
    org_arg: Option<String>,
}

impl CommandBase<'_> {
    pub fn new(user_config: &mut UserConfig, org_arg: Option<String>) -> CommandBase {
        CommandBase {
            user_config,
            app_config: OnceCell::new(),
            org_arg,
        }
    }

    pub fn api_client(&self) -> APIClient {
        let url = self.user_config.get_url();
        APIClient::new(url)
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

    pub fn get_org(&self) -> Result<String> {
        let org_name = if self.org_arg.is_some() {
            self.org_arg.clone().unwrap()
        } else {
            match self.user_config.get_default_org() {
                Some(cfg) => cfg.to_string(),
                None => return Err(anyhow!("Either set a default org in the config or provide one via --org"))
            }
        };
        Ok(org_name)
    }
}
