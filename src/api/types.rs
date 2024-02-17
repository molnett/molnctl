use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Organization {
    pub id: String,
    pub name: String,
    billing_email: String,
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

#[derive(Serialize, Deserialize, Debug, Tabled)]
pub struct Service {
    name: String,
    image: String,
    container_port: u16,
}
