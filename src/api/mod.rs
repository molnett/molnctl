use anyhow::{anyhow, Context};
use reqwest::{blocking::Response, StatusCode};
use std::collections::HashMap;

use self::types::*;

pub mod types;

pub struct APIClient {
    client: reqwest::blocking::Client,
    base_url: String,
    user_agent: String,
}

impl APIClient {
    pub fn new(base_url: impl AsRef<str>) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            base_url: base_url.as_ref().to_string(),
            user_agent: format!("molnctl/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    pub fn get_tenants(&self, token: &str) -> Result<ListTenantsResponse, reqwest::Error> {
        let url = format!("{}/tenants", self.base_url);
        let response = self.get(&url, token)?.error_for_status()?;
        response.json()
    }

    pub fn get_projects(
        &self,
        token: &str,
        tenant_name: &str,
    ) -> Result<ListProjectsResponse, reqwest::Error> {
        let url = format!("{}/tenants/{}/projects", self.base_url, tenant_name);
        let response = self.get(&url, token)?.error_for_status()?;
        response.json()
    }

    pub fn get_project(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
    ) -> anyhow::Result<Project> {
        let url = format!(
            "{}/tenants/{}/projects/{}",
            self.base_url, tenant_name, project_name
        );
        let response = self.get(&url, token)?.error_for_status()?;
        match response.status() {
            StatusCode::OK => Ok(serde_json::from_str(&response.text()?)
                .with_context(|| "Failed to deserialize project")?),
            StatusCode::UNAUTHORIZED => Err(anyhow!("Unauthorized, please login first")),
            StatusCode::NOT_FOUND => Err(anyhow!("Project does not exist")),
            _ => Err(anyhow!(
                "Failed to get project. API returned {} {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn create_project(
        &self,
        token: &str,
        tenant_name: &str,
        name: &str,
    ) -> anyhow::Result<Project> {
        let url = format!("{}/tenants/{}/projects", self.base_url, tenant_name);
        let mut body = HashMap::new();
        body.insert("name", name);
        let response = self.post(&url, token, &body)?;
        match response.status() {
            StatusCode::CREATED => Ok(serde_json::from_str(&response.text()?)
                .with_context(|| "Failed to deserialize project")?),
            StatusCode::UNAUTHORIZED => Err(anyhow!("Unauthorized, please login first")),
            StatusCode::CONFLICT => Err(anyhow!("Project already exists")),
            StatusCode::NOT_FOUND => Err(anyhow!("Tenant does not exist")),
            StatusCode::BAD_REQUEST => Err(anyhow!("Bad request: {}", response.text()?)),
            _ => Err(anyhow!(
                "Failed to create project. API returned {} {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn delete_project(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
    ) -> anyhow::Result<()> {
        let url = format!(
            "{}/tenants/{}/projects/{}",
            self.base_url, tenant_name, project_name
        );
        let response = self.delete(&url, token)?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(anyhow!("Project does not exist")),
            _ => Err(anyhow!(
                "Failed to delete project. API returned {} {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn get_environments(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
    ) -> anyhow::Result<ListEnvironmentsResponse> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments",
            self.base_url, tenant_name, project_name
        );
        let response = self.get(&url, token)?;
        match response.status() {
            StatusCode::OK => {
                let text = &response.text()?;
                Ok(serde_json::from_str(text)
                    .with_context(|| "Failed to deserialize environments")?)
            }
            StatusCode::UNAUTHORIZED => Err(anyhow!("Unauthorized, please login first")),
            StatusCode::NOT_FOUND => Err(anyhow!("Project does not exist")),
            _ => Err(anyhow!(
                "Failed to get environments. API returned {} {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn create_environment(
        &self,
        token: &str,
        name: &str,
        tenant_name: &str,
        project_name: &str,
        copy_from: Option<&str>,
    ) -> anyhow::Result<CreateEnvironmentResponse> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments",
            self.base_url, tenant_name, project_name
        );
        let mut body = HashMap::new();
        body.insert("name", name);
        if let Some(copy_from) = copy_from {
            body.insert("copy_from", copy_from);
        }
        let response = self.post(&url, token, &body)?;
        match response.status() {
            StatusCode::CREATED => Ok(serde_json::from_str(&response.text()?)
                .with_context(|| "Failed to deserialize env")?),
            StatusCode::UNAUTHORIZED => Err(anyhow!("Unauthorized, please login first")),
            StatusCode::CONFLICT => Err(anyhow!("Environment already exists")),
            StatusCode::NOT_FOUND => Err(anyhow!("Project does not exist")),
            StatusCode::BAD_REQUEST => Err(anyhow!("Bad request: {}", response.text()?)),
            _ => Err(anyhow!(
                "Failed to create environment. API returned {} - {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn delete_environment(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
        name: &str,
    ) -> anyhow::Result<()> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments/{}",
            self.base_url, tenant_name, project_name, name
        );
        let response = self.delete(&url, token)?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(anyhow!("Environment does not exist")),
            _ => Err(anyhow!(
                "Failed to delete environment. API returned {} - {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn get_service(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
        env_name: &str,
        name: &str,
    ) -> anyhow::Result<Option<ComposeService>> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments/{}/services/{}",
            self.base_url, tenant_name, project_name, env_name, name
        );
        let response = self.get(&url, token)?;
        match response.status() {
            StatusCode::OK => Ok(serde_json::from_str(&response.text()?)
                .with_context(|| "Failed to deserialize service")?),
            StatusCode::UNAUTHORIZED => Err(anyhow!("Unauthorized, please login first")),
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(anyhow!(
                "Failed to get service. API returned {} {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn get_services(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
        env_name: &str,
    ) -> anyhow::Result<ListServicesResponse> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments/{}/services",
            self.base_url, tenant_name, project_name, env_name
        );
        let response = self.get(&url, token)?.error_for_status()?;
        serde_json::from_str(&response.text()?).with_context(|| "Failed to deserialize response")
    }

    pub fn deploy_service(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
        env_name: &str,
        service: &ComposeService,
    ) -> anyhow::Result<DeployServiceResponse> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments/{}/services",
            self.base_url, tenant_name, project_name, env_name
        );
        let body = serde_json::to_string(&service)?;
        let response = self.post_str(&url, token, body)?;
        let status = response.status();
        let text = response.text()?;
        match status {
            StatusCode::CREATED => {
                Ok(serde_json::from_str(&text).with_context(|| "Failed to deserialize service")?)
            }
            StatusCode::UNAUTHORIZED => Err(anyhow!("Unauthorized, please login first")),
            StatusCode::NOT_FOUND => Err(anyhow!("Org or environment not found")),
            StatusCode::BAD_REQUEST => Err(anyhow!("Bad request: {}", text)),
            _ => Err(anyhow!(
                "Failed to deploy service. API returned {} - {}",
                status,
                text
            )),
        }
    }

    pub fn delete_service(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
        env_name: &str,
        svc_name: &str,
    ) -> anyhow::Result<()> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments/{}/services/{}",
            self.base_url, tenant_name, project_name, env_name, svc_name
        );
        let response = self.delete(&url, token)?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(anyhow!("Service does not exist")),
            _ => Err(anyhow!(
                "Failed to delete service. API returned {} - {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn get_secrets(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
        env_name: &str,
    ) -> anyhow::Result<ListSecretsResponse> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments/{}/secrets",
            self.base_url, tenant_name, project_name, env_name
        );
        let response = self.get(&url, token)?;
        match response.status() {
            StatusCode::OK => Ok(serde_json::from_str(&response.text()?)
                .with_context(|| "Failed to deserialize secrets list")?),
            StatusCode::UNAUTHORIZED => Err(anyhow!("Unauthorized, please login first")),
            StatusCode::NOT_FOUND => Err(anyhow!("Tenant or project does not exist")),
            _ => Err(anyhow!(
                "Failed to get secrets. API returned {} - {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn create_secret(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
        env_name: &str,
        name: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments/{}/secrets/{}",
            self.base_url, tenant_name, project_name, env_name, name
        );
        let mut body = HashMap::new();
        body.insert("value", value);
        let response = self.put(&url, token, &body)?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(anyhow!("Unauthorized, please login first")),
            StatusCode::NOT_FOUND => Err(anyhow!("Tenant or project does not exist")),
            _ => Err(anyhow!(
                "Failed to create secret. API returned {} - {}",
                response.status(),
                response.text()?
            )),
        }
    }

    pub fn delete_secret(
        &self,
        token: &str,
        tenant_name: &str,
        project_name: &str,
        env_name: &str,
        secret_name: &str,
    ) -> anyhow::Result<()> {
        let url = format!(
            "{}/tenants/{}/projects/{}/environments/{}/secrets/{}",
            self.base_url, tenant_name, project_name, env_name, secret_name
        );
        let response = self.delete(&url, token)?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(anyhow!("Secret does not exist")),
            _ => Err(anyhow!(
                "Failed to delete secret. API returned {} - {}",
                response.status(),
                response.text()?
            )),
        }
    }

    fn get(&self, url: &str, token: &str) -> Result<Response, reqwest::Error> {
        return self
            .client
            .get(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send();
    }

    fn put(
        &self,
        url: &str,
        token: &str,
        body: &HashMap<&str, &str>,
    ) -> Result<Response, reqwest::Error> {
        return self
            .client
            .put(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send();
    }

    fn post(
        &self,
        url: &str,
        token: &str,
        body: &HashMap<&str, &str>,
    ) -> Result<Response, reqwest::Error> {
        return self
            .client
            .post(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send();
    }

    fn post_str(&self, url: &str, token: &str, body: String) -> Result<Response, reqwest::Error> {
        return self
            .client
            .post(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(body)
            .send();
    }

    fn delete(&self, url: &str, token: &str) -> Result<Response, reqwest::Error> {
        return self
            .client
            .delete(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send();
    }
}
