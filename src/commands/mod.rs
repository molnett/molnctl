use anyhow::{anyhow, Result};

use crate::{api::APIClient, config::user::UserConfig};

pub mod auth;
pub mod environments;
pub mod projects;
pub mod secrets;
pub mod services;
pub mod tenants;

pub struct CommandBase<'a> {
    user_config: &'a mut UserConfig,
    tenant_arg: Option<String>,
    project_arg: Option<String>,
}

impl CommandBase<'_> {
    pub fn new(
        user_config: &mut UserConfig,
        tenant_arg: Option<String>,
        project_arg: Option<String>,
    ) -> CommandBase {
        CommandBase {
            user_config,
            tenant_arg,
            project_arg,
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

    pub fn get_tenant(&self) -> Result<String> {
        let tenant_name = if self.tenant_arg.is_some() {
            self.tenant_arg.clone().unwrap()
        } else {
            match self.user_config.get_default_tenant() {
                Some(cfg) => cfg.to_string(),
                None => {
                    return Err(anyhow!(
                        "Either switch to a tenant with `molnctl tenant switch` or provide one via --tenant"
                    ))
                }
            }
        };
        Ok(tenant_name)
    }

    pub fn get_project(&self) -> Result<String> {
        let project_name = if self.project_arg.is_some() {
            self.project_arg.clone().unwrap()
        } else {
            match self.user_config.get_default_project() {
                Some(cfg) => cfg.to_string(),
                None => {
                    return Err(anyhow!(
                        "Either switch to a project with `molnctl project switch` or provide one via --project"
                    ))
                }
            }
        };
        Ok(project_name)
    }
}
