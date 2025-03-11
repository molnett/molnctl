use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};
use tabled::Tabled;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub billing_email: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListTenantsResponse {
    pub tenants: Vec<Tenant>,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub tenant_id: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct CreateProjectRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct CreateProjectResponse {
    pub id: String,
    pub name: String,
    pub tenant_id: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListEnvironmentsResponse {
    pub environments: Vec<Environment>,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct CreateEnvironmentResponse {
    pub name: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub copy_from: DisplayOption<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListServicesResponse {
    pub services: Vec<Service>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Value {
    String(String),
    SecretRef {
        #[serde(rename = "secretRef")]
        secret_ref: String,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct EnvironmentValue {
    pub is_secret: bool,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Tabled, Clone, PartialEq)]
pub struct Service {
    pub name: String,
    pub image: String,
    pub container_port: u16,
    #[serde(default, skip_serializing_if = "is_default")]
    pub env: DisplayOption<DisplayHashMap<String>>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub secrets: DisplayOption<DisplayHashMap<String>>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub command: DisplayOption<DisplayVec<String>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ComposeService {
    pub name: String,
    pub image: String,
    pub ports: Vec<Port>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub environment: DisplayOption<DisplayHashMap<String>>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub secrets: DisplayOption<DisplayHashMap<String>>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub command: DisplayOption<DisplayVec<String>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Port {
    pub target: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DeployServiceResponse {
    pub id: String,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub start_time: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_time: Option<OffsetDateTime>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListSecretsResponse {
    pub secrets: Vec<Secret>,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Secret {
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct DisplayHashMap<T>(pub IndexMap<String, T>);

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct DisplayOption<T>(pub Option<T>);

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct DisplayVec<T>(pub Vec<T>);

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

impl<T: Display> Display for DisplayOption<DisplayHashMap<T>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.0.is_none() {
            return Ok(());
        }

        let hashmap = self.0.as_ref().unwrap();
        let mut entries = hashmap.0.iter().peekable();

        while let Some((key, value)) = entries.next() {
            write!(f, "{}: {}", key, value)?;

            if entries.peek().is_some() {
                write!(f, ", ")?;
            }
        }

        Ok(())
    }
}

impl<T: Display> Display for DisplayOption<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.0 {
            Some(value) => write!(f, "{}", value),
            None => Ok(()),
        }
    }
}

impl<T: Display> Display for DisplayVec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.0.is_empty() {
            Ok(())
        } else {
            let strings: Vec<String> = self.0.iter().map(|x| x.to_string()).collect();
            write!(f, "{}", strings.join(" "))
        }
    }
}
