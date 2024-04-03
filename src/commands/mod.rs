use anyhow::{anyhow, Result};

use crate::{
    api::APIClient,
    config::user::UserConfig,
};

pub mod auth;
pub mod environments;
pub mod orgs;
pub mod secrets;
pub mod services;

pub struct CommandBase<'a> {
    user_config: &'a mut UserConfig,
    org_arg: Option<String>,
}

impl CommandBase<'_> {
    pub fn new(user_config: &mut UserConfig, org_arg: Option<String>) -> CommandBase {
        CommandBase {
            user_config,
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
