use std::process::Command;
use std::fs;
use tempfile::TempDir;


/// Test that the build command can successfully build a simple Docker image
#[test]
fn test_build_simple_image() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    
    // Create a simple Dockerfile
    let dockerfile_content = r#"FROM alpine:latest
RUN echo "Hello from molnctl build test"
CMD ["echo", "Build test passed!"]
"#;
    
    fs::write(temp_path.join("Dockerfile"), dockerfile_content)
        .expect("Failed to write Dockerfile");
    
    // Run the build command using the built binary
    let output = Command::new(env!("CARGO_BIN_EXE_molnctl"))
        .arg("build")
        .arg("--tag")
        .arg("molnctl-test:latest")
        .arg("--context")
        .arg(temp_path.to_str().unwrap())
        .arg("--verbose")
        .output()
        .expect("Failed to run molnctl build");
    
    // Check that the build was successful
    if !output.status.success() {
        panic!(
            "Build failed with exit code: {}\nStdout: {}\nStderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    
    // Verify the image was created in Docker
    let docker_output = Command::new("docker")
        .arg("images")
        .arg("--format")
        .arg("{{.Repository}}:{{.Tag}}")
        .arg("molnctl-test:latest")
        .output()
        .expect("Failed to run docker images");
    
    let images_list = String::from_utf8_lossy(&docker_output.stdout);
    assert!(
        images_list.contains("molnctl-test:latest"),
        "Built image not found in Docker. Docker output: {}",
        images_list
    );
    
    // Clean up - remove the test image
    let _ = Command::new("docker")
        .arg("rmi")
        .arg("molnctl-test:latest")
        .output();
}

/// Test that the build command handles .dockerignore properly
#[test]
fn test_build_with_dockerignore() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    
    // Create a simple Dockerfile
    let dockerfile_content = r#"FROM alpine:latest
COPY . /app
RUN ls -la /app
CMD ["echo", "Dockerignore test passed!"]
"#;
    
    fs::write(temp_path.join("Dockerfile"), dockerfile_content)
        .expect("Failed to write Dockerfile");
    
    // Create some test files
    fs::write(temp_path.join("included.txt"), "This should be included")
        .expect("Failed to write included.txt");
    fs::write(temp_path.join("excluded.txt"), "This should be excluded")
        .expect("Failed to write excluded.txt");
    
    // Create .dockerignore
    let dockerignore_content = r#"excluded.txt
*.log
.git
"#;
    
    fs::write(temp_path.join(".dockerignore"), dockerignore_content)
        .expect("Failed to write .dockerignore");
    
    // Run the build command
    let output = Command::new(env!("CARGO_BIN_EXE_molnctl"))
        .arg("build")
        .arg("--tag")
        .arg("molnctl-ignore-test:latest")
        .arg("--context")
        .arg(temp_path.to_str().unwrap())
        .arg("--verbose")
        .output()
        .expect("Failed to run molnctl build");
    
    // Check that the build was successful
    if !output.status.success() {
        panic!(
            "Build failed with exit code: {}\nStdout: {}\nStderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    
    // Verify the image was created
    let docker_output = Command::new("docker")
        .arg("images")
        .arg("--format")
        .arg("{{.Repository}}:{{.Tag}}")
        .arg("molnctl-ignore-test:latest")
        .output()
        .expect("Failed to run docker images");
    
    let images_list = String::from_utf8_lossy(&docker_output.stdout);
    assert!(
        images_list.contains("molnctl-ignore-test:latest"),
        "Built image not found in Docker. Docker output: {}",
        images_list
    );
    
    // Clean up - remove the test image
    let _ = Command::new("docker")
        .arg("rmi")
        .arg("molnctl-ignore-test:latest")
        .output();
}

/// Test that the build command handles different platforms
#[test]
fn test_build_with_platform() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    
    // Create a simple Dockerfile
    let dockerfile_content = r#"FROM alpine:latest
RUN echo "Platform test"
CMD ["echo", "Platform test passed!"]
"#;
    
    fs::write(temp_path.join("Dockerfile"), dockerfile_content)
        .expect("Failed to write Dockerfile");
    
    // Run the build command with specific platform
    let output = Command::new(env!("CARGO_BIN_EXE_molnctl"))
        .arg("build")
        .arg("--tag")
        .arg("molnctl-platform-test:latest")
        .arg("--context")
        .arg(temp_path.to_str().unwrap())
        .arg("--platform")
        .arg("linux/amd64")
        .arg("--verbose")
        
        .output()
        .expect("Failed to run molnctl build");
    
    // Check that the build was successful
    if !output.status.success() {
        panic!(
            "Build failed with exit code: {}\nStdout: {}\nStderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    
    // Verify the image was created
    let docker_output = Command::new("docker")
        .arg("images")
        .arg("--format")
        .arg("{{.Repository}}:{{.Tag}}")
        .arg("molnctl-platform-test:latest")
        .output()
        .expect("Failed to run docker images");
    
    let images_list = String::from_utf8_lossy(&docker_output.stdout);
    assert!(
        images_list.contains("molnctl-platform-test:latest"),
        "Built image not found in Docker. Docker output: {}",
        images_list
    );
    
    // Clean up - remove the test image
    let _ = Command::new("docker")
        .arg("rmi")
        .arg("molnctl-platform-test:latest")
        .output();
}

/// Test that the build command validates Dockerfile existence
#[test]
fn test_build_missing_dockerfile() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    
    // Run the build command without a Dockerfile
    let output = Command::new(env!("CARGO_BIN_EXE_molnctl"))
        .arg("build")
        .arg("--tag")
        .arg("molnctl-missing-test:latest")
        .arg("--context")
        .arg(temp_path.to_str().unwrap())
        .output()
        .expect("Failed to run molnctl build");
    
    // Check that the build failed as expected
    assert!(
        !output.status.success(),
        "Build should have failed due to missing Dockerfile"
    );
    
    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Dockerfile not found"),
        "Error message should mention missing Dockerfile. Stderr: {}",
        stderr_output
    );
}

/// Test build context creation and tar archive functionality
#[test]
fn test_build_context_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    
    // Create a more complex directory structure
    let subdir = temp_path.join("subdir");
    fs::create_dir(&subdir).expect("Failed to create subdir");
    
    let dockerfile_content = r#"FROM alpine:latest
COPY . /app
RUN find /app -type f -name "*.txt" | sort
CMD ["echo", "Context test passed!"]
"#;
    
    fs::write(temp_path.join("Dockerfile"), dockerfile_content)
        .expect("Failed to write Dockerfile");
    
    fs::write(temp_path.join("root.txt"), "Root file")
        .expect("Failed to write root.txt");
    
    fs::write(subdir.join("nested.txt"), "Nested file")
        .expect("Failed to write nested.txt");
    
    // Run the build command
    let output = Command::new(env!("CARGO_BIN_EXE_molnctl"))
        .arg("build")
        .arg("--tag")
        .arg("molnctl-context-test:latest")
        .arg("--context")
        .arg(temp_path.to_str().unwrap())
        .arg("--verbose")
        .output()
        .expect("Failed to run molnctl build");
    
    // Check that the build was successful
    if !output.status.success() {
        panic!(
            "Build failed with exit code: {}\nStdout: {}\nStderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    
    // Verify the image was created
    let docker_output = Command::new("docker")
        .arg("images")
        .arg("--format")
        .arg("{{.Repository}}:{{.Tag}}")
        .arg("molnctl-context-test:latest")
        .output()
        .expect("Failed to run docker images");
    
    let images_list = String::from_utf8_lossy(&docker_output.stdout);
    assert!(
        images_list.contains("molnctl-context-test:latest"),
        "Built image not found in Docker. Docker output: {}",
        images_list
    );
    
    // Clean up - remove the test image
    let _ = Command::new("docker")
        .arg("rmi")
        .arg("molnctl-context-test:latest")
        .output();
}