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
pub mod projects;

pub struct CommandBase {
    org_arg: Option<String>,
}

impl CommandBase {
    pub fn new(org_arg: Option<String>) -> CommandBase {
        CommandBase {
            org_arg,
        }
    }

    pub fn api_client(&self) -> APIClient {
        APIClient::new(&UserConfig::get_url())
    }

    pub fn get_token(&self) -> Result<String> {
        UserConfig::get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))
    }

    pub fn get_org(&self) -> Result<String> {
        if let Some(org) = &self.org_arg {
            return Ok(org.clone());
        }

        UserConfig::get_default_org()
            .ok_or_else(|| anyhow!("Either set a default org in the config or provide one via --org"))
    }
}
