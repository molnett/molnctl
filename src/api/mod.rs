use std::collections::HashMap;

use self::types::{ListOrganizationResponse, Organization, CreateEnvironmentResponse};

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

        let response = self.client
            .get(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send()?
            .error_for_status()?;

        response.json()
    }

    pub fn get_application(
        &self,
        token: &str,
        name: &str
    ) -> Result<ListOrganizationResponse, reqwest::Error> {
        let url = format!("{}/organization", self.base_url);

        let response = self.client
            .get(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send()?
            .error_for_status()?;

        response.json()
    }

    pub fn get_applications(
        &self,
        token: &str,
    ) -> Result<ListOrganizationResponse, reqwest::Error> {
        let url = format!("{}/organization", self.base_url);

        let response = self.client
            .get(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send()?
            .error_for_status()?;

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

        let response = self
            .client
            .post(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?
            .error_for_status()?;

        response.json()
    }

    pub fn get_environments(
        &self,
        token: &str,
        org_name: &str
    ) -> Result<Vec<String>, reqwest::Error> {
        let url = format!("{}/orgs/{}/envs", self.base_url, org_name);

        let response = self.client
            .get(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send()?
            .error_for_status()?;

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

        let response = self
            .client
            .post(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?
            .error_for_status()?;

        response.json()
    }

    pub fn initialize_application(&self) -> Result<(), reqwest::Error> {
        let url = format!("{}/application", self.base_url);

        let response = self.client.post(url).send()?.error_for_status()?;

        response.json()
    }
}
