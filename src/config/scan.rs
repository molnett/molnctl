use std::path::Path;

#[derive(Debug)]
pub enum ApplicationType {
    Rust,
    Unknown,
}

pub fn scan_directory_for_type() -> ApplicationType {
    let directory = Path::new(".");
    println!("Scanning directory: {}", directory.display());

    let application_types = vec![application_rust];

    for application_type in application_types {
        if let Some(application_type) = application_type(directory) {
            return application_type;
        }
    }

    return ApplicationType::Unknown;
}

pub fn application_rust(directory: &Path) -> Option<ApplicationType> {
    let is_rust = directory.join("Cargo.toml").exists();

    if is_rust {
        Some(ApplicationType::Rust)
    } else {
        None
    }
}
