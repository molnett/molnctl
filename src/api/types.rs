use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};
use tabled::Tabled;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub billing_email: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListOrganizationResponse {
    pub organizations: Vec<Organization>,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Environment {
    pub name: String,
    pub organization_id: String,
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

#[derive(Serialize, Deserialize, Debug, Tabled, Clone, PartialEq)]
pub struct Service {
    pub name: String,
    pub image: String,
    pub container_port: u16,
    #[serde(default, skip_serializing_if = "is_default")]
    pub env: DisplayOption<DisplayHashMap>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub secrets: DisplayOption<DisplayHashMap>,
}

/*#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DeployServiceResponse {
    pub id: String,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub start_time: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_time: Option<OffsetDateTime>,
    pub error: Option<String>,
}
*/
#[derive(Serialize, Deserialize, Debug)]
pub struct ListSecretsResponse {
    pub secrets: Vec<Secret>,
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Secret {
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct DisplayHashMap(pub IndexMap<String, String>);

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct DisplayOption<T>(pub Option<T>);

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

impl Display for DisplayOption<DisplayHashMap> {
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
