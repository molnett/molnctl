use std::collections::HashMap;
use reqwest::blocking::Response;

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
    ) -> Result<Service, reqwest::Error> {
        let url = format!("{}/orgs/{}/envs/{}/svcs", self.base_url, org_name, env_name);
        let mut body: HashMap<&str, &str> = HashMap::new();
        let port_str = &format!("{}", service.container_port);
        body.insert("container_port", port_str);
        body.insert("name", &service.name);
        body.insert("image", &service.image);
        let response = self.post(&url, token, &body)?;
        response.json()
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
}
