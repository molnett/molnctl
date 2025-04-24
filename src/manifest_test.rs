use anyhow::Result;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

use crate::api::types::{
    ComposeService, Container, DisplayVec, NonComposeManifest, Port, Volume, VolumeMount,
};
use crate::commands::services::{read_manifest, ComposeFile};

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to write a YAML file to a temporary directory
    fn write_temp_yaml<T: serde::Serialize>(
        content: &T,
        dir: &tempfile::TempDir,
        filename: &str,
    ) -> Result<String> {
        let file_path = dir.path().join(filename);
        let yaml = serde_yaml::to_string(content)?;
        let mut file = File::create(&file_path)?;
        file.write_all(yaml.as_bytes())?;
        Ok(file_path.to_string_lossy().to_string())
    }

    // Test reading manifests with both formats
    #[test]
    fn test_read_manifest_formats() -> Result<()> {
        // Create a temporary directory that will stay in scope
        let temp_dir = tempdir()?;

        // Create a manifest with services field without containers (old format)
        let old_format = NonComposeManifest {
            version: 1,
            services: vec![
                Container {
                    name: "web".to_string(),
                    image: "nginx:latest".to_string(),
                    container_type: "".to_string(),
                    shared_volume_path: "".to_string(),
                    command: vec![],
                    environment: IndexMap::new(),
                    secrets: IndexMap::new(),
                    ports: vec![Port {
                        target: 80,
                        publish: Some(true),
                    }],
                    volume_mounts: vec![],
                },
                Container {
                    name: "api".to_string(),
                    image: "node:14".to_string(),
                    container_type: "".to_string(),
                    shared_volume_path: "".to_string(),
                    command: vec!["node".to_string(), "server.js".to_string()],
                    environment: {
                        let mut env = IndexMap::new();
                        env.insert("NODE_ENV".to_string(), "production".to_string());
                        env
                    },
                    secrets: IndexMap::new(),
                    ports: vec![Port {
                        target: 3000,
                        publish: Some(true),
                    }],
                    volume_mounts: vec![],
                },
            ],
        };

        // Create a manifest with containers field (new format)
        let new_format = ComposeFile {
            version: 1,
            services: vec![
                ComposeService {
                    name: "web".to_string(),
                    volumes: DisplayVec(vec![]),
                    containers: DisplayVec(vec![Container {
                        name: "main".to_string(),
                        image: "nginx:latest".to_string(),
                        container_type: "main".to_string(),
                        shared_volume_path: "".to_string(),
                        command: vec![],
                        environment: IndexMap::new(),
                        secrets: IndexMap::new(),
                        ports: vec![Port {
                            target: 80,
                            publish: Some(true),
                        }],
                        volume_mounts: vec![],
                    }]),
                },
                ComposeService {
                    name: "api".to_string(),
                    volumes: DisplayVec(vec![]),
                    containers: DisplayVec(vec![
                        Container {
                            name: "main".to_string(),
                            image: "node:14".to_string(),
                            container_type: "main".to_string(),
                            shared_volume_path: "".to_string(),
                            command: vec!["node".to_string(), "server.js".to_string()],
                            environment: {
                                let mut env = IndexMap::new();
                                env.insert("NODE_ENV".to_string(), "production".to_string());
                                env
                            },
                            secrets: IndexMap::new(),
                            ports: vec![Port {
                                target: 3000,
                                publish: Some(true),
                            }],
                            volume_mounts: vec![],
                        },
                        Container {
                            name: "redis".to_string(),
                            image: "redis:alpine".to_string(),
                            container_type: "cache".to_string(),
                            shared_volume_path: "/data".to_string(),
                            command: vec![],
                            environment: IndexMap::new(),
                            secrets: IndexMap::new(),
                            ports: vec![Port {
                                target: 6379,
                                publish: None,
                            }],
                            volume_mounts: vec![],
                        },
                    ]),
                },
            ],
        };

        // Write both formats to temporary files
        let old_format_path = write_temp_yaml(&old_format, &temp_dir, "old_format.yaml")?;
        let new_format_path = write_temp_yaml(&new_format, &temp_dir, "new_format.yaml")?;

        // Read and validate old format
        let read_old = read_manifest(&old_format_path)?;
        assert_eq!(read_old.version, 1);
        assert_eq!(read_old.services.len(), 2);

        // Old format should be converted to new format
        let web_service = read_old.services.iter().find(|s| s.name == "web").unwrap();
        assert_eq!(web_service.containers.0.len(), 1);
        assert_eq!(web_service.containers.0[0].name, "main");
        assert_eq!(web_service.containers.0[0].image, "nginx:latest");

        let api_service = read_old.services.iter().find(|s| s.name == "api").unwrap();
        assert_eq!(api_service.containers.0.len(), 1);
        assert_eq!(api_service.containers.0[0].name, "main");
        assert_eq!(api_service.containers.0[0].image, "node:14");
        assert_eq!(
            api_service.containers.0[0].command,
            vec!["node", "server.js"]
        );
        assert_eq!(
            api_service.containers.0[0]
                .environment
                .get("NODE_ENV")
                .unwrap(),
            "production"
        );

        // Read and validate new format
        let read_new = read_manifest(&new_format_path)?;
        assert_eq!(read_new.version, 1);
        assert_eq!(read_new.services.len(), 2);

        let web_service = read_new.services.iter().find(|s| s.name == "web").unwrap();
        assert_eq!(web_service.containers.0.len(), 1);
        assert_eq!(web_service.containers.0[0].image, "nginx:latest");

        let api_service = read_new.services.iter().find(|s| s.name == "api").unwrap();
        assert_eq!(api_service.containers.0.len(), 2);
        assert_eq!(api_service.containers.0[0].name, "main");
        assert_eq!(api_service.containers.0[0].image, "node:14");
        assert_eq!(api_service.containers.0[1].name, "redis");
        assert_eq!(api_service.containers.0[1].image, "redis:alpine");
        assert_eq!(api_service.containers.0[1].container_type, "cache");
        assert_eq!(api_service.containers.0[1].shared_volume_path, "/data");

        Ok(())
    }

    // Test diffing both manifest formats
    #[test]
    fn test_diff_manifest_formats() -> Result<()> {
        // Create a temporary directory that will stay in scope
        let temp_dir = tempdir()?;

        // Create a simple old format manifest - will be converted during read
        let old_format = NonComposeManifest {
            version: 1,
            services: vec![Container {
                name: "app".to_string(),
                image: "app:v1".to_string(),
                container_type: "".to_string(),
                shared_volume_path: "".to_string(),
                command: vec![],
                environment: {
                    let mut env = IndexMap::new();
                    env.insert("DEBUG".to_string(), "false".to_string());
                    env
                },
                secrets: IndexMap::new(),
                ports: vec![Port {
                    target: 8080,
                    publish: Some(true),
                }],
                volume_mounts: vec![],
            }],
        };

        // Create a new format with changes
        let new_format = ComposeFile {
            version: 1,
            services: vec![ComposeService {
                name: "app".to_string(),
                volumes: DisplayVec(vec![]),
                containers: DisplayVec(vec![
                    Container {
                        name: "main".to_string(),
                        image: "app:v2".to_string(), // Changed image version
                        container_type: "main".to_string(),
                        shared_volume_path: "".to_string(),
                        command: vec![],
                        environment: {
                            let mut env = IndexMap::new();
                            env.insert("DEBUG".to_string(), "true".to_string()); // Changed env var
                            env
                        },
                        secrets: IndexMap::new(),
                        ports: vec![Port {
                            target: 8080,
                            publish: Some(true),
                        }],
                        volume_mounts: vec![],
                    },
                    Container {
                        // Added sidecar container
                        name: "sidecar".to_string(),
                        image: "sidecar:latest".to_string(),
                        container_type: "helper".to_string(),
                        shared_volume_path: "/shared".to_string(),
                        command: vec![],
                        environment: IndexMap::new(),
                        secrets: IndexMap::new(),
                        ports: vec![],
                        volume_mounts: vec![],
                    },
                ]),
            }],
        };

        // Write both formats to temporary files
        let old_format_path = write_temp_yaml(&old_format, &temp_dir, "old_format_diff.yaml")?;
        let new_format_path = write_temp_yaml(&new_format, &temp_dir, "new_format_diff.yaml")?;

        // Read both manifests
        let read_old = read_manifest(&old_format_path)?;
        let read_new = read_manifest(&new_format_path)?;

        // Make sure the old format was properly read and converted
        assert_eq!(read_old.services.len(), 1);
        let old_app = read_old.services.iter().find(|s| s.name == "app").unwrap();
        assert_eq!(old_app.containers.0.len(), 1);
        assert_eq!(old_app.containers.0[0].name, "main");
        assert_eq!(old_app.containers.0[0].image, "app:v1");

        // Make sure the DEBUG env variable is present in the old format
        assert!(old_app.containers.0[0].environment.contains_key("DEBUG"));
        assert_eq!(
            old_app.containers.0[0].environment.get("DEBUG").unwrap(),
            "false"
        );

        // Make sure the new format is properly read
        assert_eq!(read_new.services.len(), 1);
        let new_app = read_new.services.iter().find(|s| s.name == "app").unwrap();
        assert_eq!(new_app.containers.0.len(), 2);
        assert_eq!(new_app.containers.0[0].name, "main");
        assert_eq!(new_app.containers.0[0].image, "app:v2");
        assert_eq!(
            new_app.containers.0[0].environment.get("DEBUG").unwrap(),
            "true"
        );

        // Convert to YAML strings for diffing
        let old_yaml = serde_yaml::to_string(&read_old)?;
        let new_yaml = serde_yaml::to_string(&read_new)?;

        // Create a changeset to see differences
        let changeset = difference::Changeset::new(&old_yaml, &new_yaml, "\n");

        // Examine each diff to look for key changes
        let mut found_image_change = false;
        let mut found_debug_change = false;
        let mut found_sidecar_addition = false;

        // Print all diffs for debugging
        println!("Changes detected:");
        for diff in &changeset.diffs {
            match diff {
                difference::Difference::Same(_) => {}
                difference::Difference::Add(added) => {
                    println!("+ {}", added);
                    if added.contains("sidecar:latest") {
                        found_sidecar_addition = true;
                    }
                    if added.contains("DEBUG: \"true\"") {
                        found_debug_change = true;
                    }
                }
                difference::Difference::Rem(removed) => {
                    println!("- {}", removed);
                    if removed.contains("app:v1") {
                        found_image_change = true;
                    }
                    if removed.contains("DEBUG: \'false\'") {
                        found_debug_change = true;
                    }
                }
            }
        }

        assert!(found_image_change, "Failed to detect image version change");
        assert!(found_debug_change, "Failed to detect DEBUG env var change");
        assert!(
            found_sidecar_addition,
            "Failed to detect sidecar container addition"
        );

        Ok(())
    }

    // Test volumes and volume mounts
    #[test]
    fn test_volumes_and_mounts() -> Result<()> {
        // Create a temporary directory
        let temp_dir = tempdir()?;

        // Create a manifest with volumes and volume mounts
        let manifest = ComposeFile {
            version: 1,
            services: vec![ComposeService {
                name: "app".to_string(),
                volumes: DisplayVec(vec![
                    Volume {
                        name: "app_data".to_string(),
                    },
                    Volume {
                        name: "shared_logs".to_string(),
                    },
                ]),
                containers: DisplayVec(vec![
                    Container {
                        name: "main".to_string(),
                        image: "app:latest".to_string(),
                        container_type: "main".to_string(),
                        shared_volume_path: "/app/data".to_string(),
                        command: vec![],
                        environment: IndexMap::new(),
                        secrets: IndexMap::new(),
                        ports: vec![],
                        volume_mounts: vec![
                            VolumeMount {
                                volume_name: "app_data".to_string(),
                                path: "/app/data".to_string(),
                            },
                            VolumeMount {
                                volume_name: "./logs".to_string(),
                                path: "/app/logs".to_string(),
                            },
                        ],
                    },
                    Container {
                        name: "sidecar".to_string(),
                        image: "logger:latest".to_string(),
                        container_type: "helper".to_string(),
                        shared_volume_path: "/logs".to_string(),
                        command: vec![],
                        environment: IndexMap::new(),
                        secrets: IndexMap::new(),
                        ports: vec![],
                        volume_mounts: vec![
                            VolumeMount {
                                volume_name: "shared_logs".to_string(),
                                path: "/logs".to_string(),
                            },
                            VolumeMount {
                                volume_name: "./logs".to_string(),
                                path: "/backup".to_string(),
                            },
                        ],
                    },
                ]),
            }],
        };

        // Write manifest to a temporary file
        let manifest_path = write_temp_yaml(&manifest, &temp_dir, "volumes_test.yaml")?;

        // Read the manifest back
        let read_manifest = read_manifest(&manifest_path)?;

        // Verify the volumes were parsed correctly
        assert_eq!(read_manifest.services.len(), 1);
        let app_service = &read_manifest.services[0];
        assert_eq!(app_service.name, "app");

        // Check volumes at service level
        assert_eq!(app_service.volumes.0.len(), 2);
        assert!(app_service.volumes.0.iter().any(|v| v.name == "app_data"));
        assert!(app_service
            .volumes
            .0
            .iter()
            .any(|v| v.name == "shared_logs"));

        // Check main container volume mounts
        let main_container = app_service
            .containers
            .0
            .iter()
            .find(|c| c.name == "main")
            .unwrap();
        assert_eq!(main_container.volume_mounts.len(), 2);
        assert!(main_container
            .volume_mounts
            .iter()
            .any(|vm| vm.volume_name == "app_data" && vm.path == "/app/data"));
        assert!(main_container
            .volume_mounts
            .iter()
            .any(|vm| vm.volume_name == "./logs" && vm.path == "/app/logs"));
        assert_eq!(main_container.shared_volume_path, "/app/data");

        // Check sidecar container volume mounts
        let sidecar_container = app_service
            .containers
            .0
            .iter()
            .find(|c| c.name == "sidecar")
            .unwrap();
        assert_eq!(sidecar_container.volume_mounts.len(), 2);
        assert!(sidecar_container
            .volume_mounts
            .iter()
            .any(|vm| vm.volume_name == "shared_logs" && vm.path == "/logs"));
        assert!(sidecar_container
            .volume_mounts
            .iter()
            .any(|vm| vm.volume_name == "./logs" && vm.path == "/backup"));
        assert_eq!(sidecar_container.shared_volume_path, "/logs");

        Ok(())
    }
}
