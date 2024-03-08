use std::fmt::{Display, Formatter, Result};
use std::collections::HashMap;
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
    pub env: DisplayOption<DisplayHashMap>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListSecretsResponse {
    pub secrets: Vec<Secret>
}

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Secret {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DisplayHashMap(HashMap<String, String>);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DisplayOption<T>(Option<T>);

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
