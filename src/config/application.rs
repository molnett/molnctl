use camino::Utf8PathBuf;
use config::Config;

use super::{write_to_disk_yaml, Error};

#[derive(Debug)]
pub struct ApplicationConfig {
    config: ApplicationConfigInner,
    disk_config: ApplicationConfigInner,
    path: Utf8PathBuf,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct ApplicationConfigInner {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_service: Option<HttpService>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<Build>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct HttpService {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_port: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_https: Option<bool>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Build {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_image_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_image_path: Option<String>,
}

impl HttpService {
    pub fn new(container_port: u16, force_https: Option<bool>) -> Self {
        HttpService {
            container_port: Some(container_port),
            force_https,
        }
    }
}

impl Build {
    pub fn new(docker_image_path: Option<String>, docker_image_url: Option<String>) -> Self {
        if docker_image_url.is_none() && docker_image_path.is_none() {
            panic!("Either docker_image_url and docker_image_path has to be set");
        }
        if docker_image_url.is_some() && docker_image_path.is_some() {
            panic!("Both docker_image_url and docker_image_path cannot be set");
        }

        Build {
            docker_image_url,
            docker_image_path,
        }
    }
}

pub struct ApplicationConfigLoader {
    pub path: Utf8PathBuf,
}

impl ApplicationConfig {
    pub fn name(&self) -> Option<&str> {
        self.config.name.as_deref()
    }

    pub fn set_name(&mut self, name: String) -> Result<(), super::Error> {
        self.disk_config.name = Some(name.clone());
        self.config.name = Some(name.clone());

        write_to_disk_yaml(&self.path, &self.disk_config)
    }

    pub fn set_http_service(&mut self, http_service: HttpService) -> Result<(), super::Error> {
        self.disk_config.http_service = Some(http_service.clone());
        self.config.http_service = Some(http_service);

        write_to_disk_yaml(&self.path, &self.disk_config)
    }

    pub fn set_build_config(&mut self, build_config: Build) -> Result<(), super::Error> {
        self.disk_config.build = Some(build_config.clone());
        self.config.build = Some(build_config);

        write_to_disk_yaml(&self.path, &self.disk_config)
    }
}

impl ApplicationConfigLoader {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(self) -> Result<ApplicationConfig, Error> {
        let disk_config = Config::builder()
            .add_source(
                config::File::with_name(self.path.as_str())
                    .format(config::FileFormat::Yaml)
                    .required(false),
            )
            .build()?;

        let config = Config::builder().add_source(disk_config.clone()).build()?;

        Ok(ApplicationConfig {
            config: config.try_deserialize()?,
            disk_config: disk_config.try_deserialize()?,
            path: self.path,
        })
    }
}
