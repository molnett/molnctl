use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Get the path to test fixtures
fn fixtures_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Test build statistics with the Python app fixture
#[test]
fn test_build_stats_python_app() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    
    // Copy the fixture files to temp directory
    let fixtures = fixtures_path();
    let dockerfile_path = fixtures.join("dockerfiles").join("stats-test.Dockerfile");
    let app_py_path = fixtures.join("apps").join("app.py");
    let requirements_path = fixtures.join("apps").join("requirements.txt");
    
    // Copy all necessary files
    fs::copy(&dockerfile_path, temp_path.join("Dockerfile"))
        .expect("Failed to copy Dockerfile");
    fs::copy(&app_py_path, temp_path.join("app.py"))
        .expect("Failed to copy app.py");
    fs::copy(&requirements_path, temp_path.join("requirements.txt"))
        .expect("Failed to copy requirements.txt");
    
    // Run the build command using the built binary
    let output = Command::new(env!("CARGO_BIN_EXE_molnctl"))
        .arg("build")
        .arg("--image-name")
        .arg("molnctl-stats-test")
        .arg("--context")
        .arg(temp_path.to_str().unwrap())
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
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Check that build completed successfully
    assert!(
        stdout.contains("Building image:") || stdout.contains("Build completed") || stdout.contains("layers total"),
        "Build output should indicate completion. Output: {}",
        stdout
    );
    
    // Verify the image was created
    let docker_output = Command::new("docker")
        .arg("images")
        .arg("--format")
        .arg("{{.Repository}}:{{.Tag}}")
        .arg("molnctl-stats-test:*")
        .output()
        .expect("Failed to run docker images");
    
    let images_list = String::from_utf8_lossy(&docker_output.stdout);
    assert!(
        !images_list.trim().is_empty(),
        "Built image not found in Docker. Docker output: {}",
        images_list
    );
    
    // Clean up - remove the test image
    let _ = Command::new("docker")
        .arg("rmi")
        .arg("-f")
        .arg(&format!("molnctl-stats-test:{}", get_git_commit_sha()))
        .output();
}

/// Test that cache statistics are consistent between builds
#[test]
fn test_build_cache_consistency() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    
    // Copy the fixture files to temp directory
    let fixtures = fixtures_path();
    let dockerfile_path = fixtures.join("dockerfiles").join("stats-test.Dockerfile");
    let app_py_path = fixtures.join("apps").join("app.py");
    let requirements_path = fixtures.join("apps").join("requirements.txt");
    
    // Copy all necessary files
    fs::copy(&dockerfile_path, temp_path.join("Dockerfile"))
        .expect("Failed to copy Dockerfile");
    fs::copy(&app_py_path, temp_path.join("app.py"))
        .expect("Failed to copy app.py");
    fs::copy(&requirements_path, temp_path.join("requirements.txt"))
        .expect("Failed to copy requirements.txt");
    
    // First build
    let output1 = Command::new(env!("CARGO_BIN_EXE_molnctl"))
        .arg("build")
        .arg("--image-name")
        .arg("molnctl-cache-test")
        .arg("--context")
        .arg(temp_path.to_str().unwrap())
        .output()
        .expect("Failed to run first molnctl build");
    
    assert!(output1.status.success(), "First build should succeed");
    
    // Second build (should have more cache hits)
    let output2 = Command::new(env!("CARGO_BIN_EXE_molnctl"))
        .arg("build")
        .arg("--image-name")
        .arg("molnctl-cache-test")
        .arg("--context")
        .arg(temp_path.to_str().unwrap())
        .output()
        .expect("Failed to run second molnctl build");
    
    assert!(output2.status.success(), "Second build should succeed");
    
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    
    // Both builds should show the same total layer count
    let layers1 = extract_layer_count(&stdout1);
    let layers2 = extract_layer_count(&stdout2);
    
    assert_eq!(
        layers1, layers2,
        "Both builds should report the same total layer count. Build1: {}, Build2: {}",
        stdout1, stdout2
    );
    
    // Clean up
    let _ = Command::new("docker")
        .arg("rmi")
        .arg("-f")
        .arg(&format!("molnctl-cache-test:{}", get_git_commit_sha()))
        .output();
}

/// Helper function to extract layer count from build output
fn extract_layer_count(output: &str) -> Option<u32> {
    for line in output.lines() {
        if line.contains("layers total") {
            // Look for pattern like "9 layers total"
            if let Some(start) = line.find("• ") {
                let after_bullet = &line[start + 2..];
                if let Some(end) = after_bullet.find(" layers total") {
                    let number_str = &after_bullet[..end];
                    return number_str.parse().ok();
                }
            }
        }
    }
    None
}

/// Helper function to get git commit SHA
fn get_git_commit_sha() -> String {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()
        .expect("Failed to get git commit SHA");
    
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}