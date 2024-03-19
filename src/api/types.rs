use std::fmt::{Display, Formatter, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

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
pub struct CreateEnvironmentResponse {
    pub name: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListServicesResponse {
    pub services: Vec<Service>
}

#[derive(Serialize, Deserialize, Debug, Tabled, Clone, PartialEq)]
pub struct Service {
    pub name: String,
    pub image: String,
    pub container_port: u16,
    #[serde(default, skip_serializing_if = "is_default")]
    pub env: DisplayOption<DisplayHashMap>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub secrets: DisplayOption<DisplayHashMap>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListSecretsResponse {
    pub secrets: Vec<Secret>
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Secret {
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct DisplayHashMap(IndexMap<String, String>);

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct DisplayOption<T>(Option<T>);

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

impl Display for DisplayOption<DisplayHashMap> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.0.is_none() {
            return Ok(())
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
