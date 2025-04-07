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
    pub services: Vec<ComposeService>,
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NonComposeManifest {
    pub version: u16,
    pub services: Vec<Container>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Tabled)]
pub struct ComposeService {
    pub name: String,
    #[serde(default)]
    pub containers: DisplayVec<Container>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Container {
    pub name: String,
    pub image: String,
    #[serde(rename = "type", default, skip_serializing_if = "String::is_empty")]
    pub container_type: String,
    #[serde(
        rename = "shared_volume_path",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub shared_volume_path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub environment: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub secrets: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,
}

impl Display for Container {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{} ({})", self.name, self.image)
    }
}

impl Tabled for Container {
    const LENGTH: usize = 5;

    fn fields(&self) -> Vec<std::borrow::Cow<'_, str>> {
        let ports_str = if self.ports.is_empty() {
            String::new()
        } else {
            self.ports
                .iter()
                .map(|p| match p.publish {
                    Some(true) => format!("{} (published)", p.target),
                    _ => format!("{}", p.target),
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        let env_count = self.environment.len() + self.secrets.len();
        let env_display = if env_count > 0 {
            format!("{} vars", env_count)
        } else {
            String::new()
        };

        vec![
            self.name.clone().into(),
            self.image.clone().into(),
            (if !self.container_type.is_empty() {
                &self.container_type
            } else {
                "default"
            })
            .into(),
            ports_str.into(),
            env_display.into(),
        ]
    }

    fn headers() -> Vec<std::borrow::Cow<'static, str>> {
        vec![
            "NAME".into(),
            "IMAGE".into(),
            "TYPE".into(),
            "PORTS".into(),
            "ENV".into(),
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Port {
    pub target: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish: Option<bool>,
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
