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
    pub env: DisplayHashMap
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DisplayHashMap(HashMap<String, String>);

impl Display for DisplayHashMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut entries = self.0.iter().peekable();

        while let Some((key, value)) = entries.next() {
            write!(f, "{}: {}", key, value)?;

            if entries.peek().is_some() {
                write!(f, ", ")?;
            }
        }

        Ok(())
    }
}
