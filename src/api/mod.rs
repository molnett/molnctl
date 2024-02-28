use anyhow::anyhow;
use std::collections::HashMap;
use reqwest::{blocking::Response, StatusCode};

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

    pub fn get_organizations(
        &self,
        token: &str,
    ) -> Result<ListOrganizationResponse, reqwest::Error> {
        let url = format!("{}/orgs", self.base_url);
        let response = self.get(&url, token)?;
        response.json()
    }

    pub fn get_service(
        &self,
        token: &str,
        org_name: &str,
        env_name: &str,
        name: &str
    ) -> Result<Service, reqwest::Error> {
        let url = format!("{}/orgs/{}/envs/{}/svcs/{}", self.base_url, org_name, env_name, name);
        let response = self.get(&url, token)?;
        response.json()
    }

    pub fn get_services(
        &self,
        token: &str,
        org_name: &str,
        env_name: &str
    ) -> Result<ListServicesResponse, reqwest::Error> {
        let url = format!("{}/orgs/{}/envs/{}/svcs", self.base_url, org_name, env_name);
        let response = self.get(&url, token)?;
        response.json()
    }

    pub fn create_organization(
        &self,
        token: &str,
        name: &str,
        billing_email: &str,
    ) -> Result<Organization, reqwest::Error> {
        let url = format!("{}/orgs", self.base_url);
        let mut body = HashMap::new();
        body.insert("name", name);
        body.insert("billing_email", billing_email);
        let response = self.post(&url, token, &body)?;
        response.json()
    }

    pub fn get_environments(
        &self,
        token: &str,
        org_name: &str
    ) -> Result<Vec<String>, reqwest::Error> {
        let url = format!("{}/orgs/{}/envs", self.base_url, org_name);
        let response = self.get(&url, token)?;
        response.json()
    }

    pub fn create_environment(
        &self,
        token: &str,
        name: &str,
        org_name: &str
    ) -> Result<CreateEnvironmentResponse, reqwest::Error> {
        let url = format!("{}/orgs/{}/envs", self.base_url, org_name);
        let mut body = HashMap::new();
        body.insert("name", name);
        let response = self.post(&url, token, &body)?;
        response.json()
    }

    pub fn deploy_service(
        &self,
        token: &str,
        org_name: &str,
        env_name: &str,
        service: Service
    ) -> anyhow::Result<Service> {
        let url = format!("{}/orgs/{}/envs/{}/svcs", self.base_url, org_name, env_name);
        let body = serde_json::to_string(&service)?;
        let response = self.post_str(&url, token, body)?;
        Ok(serde_json::from_str(&response.text()?)?)
    }

    pub fn delete_service(
        &self,
        token: &str,
        org_name: &str,
        env_name: &str,
        svc_name: &str
    ) -> anyhow::Result<()> {
        let url = format!("{}/orgs/{}/envs/{}/svcs/{}", self.base_url, org_name, env_name, svc_name);
        let response = self.delete(&url, token)?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(anyhow!("Service does not exist")),
            _ => Err(anyhow!("Failed to delete service. API returned {} {}", response.status(), response.text()?))
        }
    }

    fn get(&self, url: &str, token: &str) -> Result<Response, reqwest::Error> {
        return self.client
            .get(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send()?
            .error_for_status();
    }

    fn post(&self, url: &str, token: &str, body: &HashMap<&str, &str>) -> Result<Response, reqwest::Error> {
        return self.client
            .post(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?
            .error_for_status();
    }

    fn post_str(&self, url: &str, token: &str, body: String) -> Result<Response, reqwest::Error> {
        return self.client
            .post(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(body)
            .send()?
            .error_for_status();
    }

    fn delete(&self, url: &str, token: &str) -> Result<Response, reqwest::Error> {
        return self.client
            .delete(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send();
    }
}
