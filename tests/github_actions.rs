use std::fs;
use std::path::Path;

/// Test that all required files exist for GitHub Actions workflows
#[test]
fn test_github_actions_files_exist() {
    let required_files = [
        ".github/workflows/docker.yml",
        "Dockerfile",
        ".dockerignore",
        "docker-compose.yml",
    ];

    for file in &required_files {
        assert!(
            Path::new(file).exists(),
            "Required file {} does not exist",
            file
        );
    }
}

/// Test that Dockerfile references existing files
#[test]
fn test_dockerfile_references_valid_files() {
    let dockerfile_content = fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

    let required_dirs = ["src", "benches", "tests", "installers"];
    let required_files = ["Cargo.toml", "Cargo.lock", "dist.toml", "deny.toml"];

    // Check that all required directories are copied
    for dir in &required_dirs {
        assert!(
            dockerfile_content.contains(&format!("COPY {} ./{}", dir, dir)),
            "Dockerfile does not copy {} directory",
            dir
        );

        // Verify directory exists
        assert!(
            Path::new(dir).exists(),
            "Directory {} referenced in Dockerfile does not exist",
            dir
        );
    }

    // Check that all required files are copied
    for file in &required_files {
        assert!(
            Path::new(file).exists(),
            "File {} referenced in Dockerfile does not exist",
            file
        );
    }
}

/// Test that GitHub Actions workflows have correct syntax
#[test]
fn test_github_workflows_syntax() {
    let workflow_dir = Path::new(".github/workflows");
    assert!(
        workflow_dir.exists(),
        ".github/workflows directory does not exist"
    );

    let entries = fs::read_dir(workflow_dir).expect("Failed to read workflows directory");

    for entry in entries {
        let entry = entry.expect("Failed to read workflow entry");
        let path = entry.path();

        if path
            .extension()
            .is_some_and(|ext| ext == "yml" || ext == "yaml")
        {
            let content = fs::read_to_string(&path).expect("Failed to read workflow file");

            // Basic YAML syntax check (no tabs, proper indentation)
            assert!(
                !content.contains('\t'),
                "Workflow file {:?} contains tabs, use spaces instead",
                path
            );

            // Check for common workflow issues
            assert!(
                content.contains("on:"),
                "Workflow file {:?} missing 'on:' trigger",
                path
            );

            assert!(
                content.contains("jobs:"),
                "Workflow file {:?} missing 'jobs:' section",
                path
            );
        }
    }
}

/// Test that SARIF file paths in workflows are correct
#[test]
fn test_sarif_file_paths() {
    let workflow_files = [".github/workflows/docker.yml"];

    for workflow_file in &workflow_files {
        if Path::new(workflow_file).exists() {
            let content = fs::read_to_string(workflow_file).expect("Failed to read workflow file");

            // Check that SARIF upload conditions are properly set
            if content.contains("upload-sarif") {
                assert!(
                    content.contains("if: always()") || content.contains("if: always() &&"),
                    "Workflow file {} has SARIF upload without proper conditions",
                    workflow_file
                );

                // Check that the SARIF file path is consistent
                assert!(
                    content.contains("trivy-results.sarif"),
                    "Workflow file {} has inconsistent SARIF file naming",
                    workflow_file
                );
            }
        }
    }
}

/// Test that Docker build context includes all necessary files
#[test]
fn test_docker_build_context() {
    let dockerfile_content = fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

    // Check that the build context includes all necessary components
    let build_context_files = ["src/", "benches/", "tests/", "Cargo.toml", "Cargo.lock"];

    for file in &build_context_files {
        if file.ends_with('/') {
            let dir_name = file.trim_end_matches('/');
            assert!(
                Path::new(dir_name).exists(),
                "Directory {} required for Docker build does not exist",
                dir_name
            );
        } else {
            assert!(
                Path::new(file).exists(),
                "File {} required for Docker build does not exist",
                file
            );
        }
    }

    // Check that the final stage copies the correct binaries
    assert!(
        dockerfile_content.contains("COPY --from=builder /app/target/release/fukura"),
        "Dockerfile does not copy fukura binary"
    );
    assert!(
        dockerfile_content.contains("COPY --from=builder /app/target/release/fuku"),
        "Dockerfile does not copy fuku binary"
    );
}

/// Test that all referenced scripts and installers exist
#[test]
fn test_scripts_and_installers_exist() {
    let dockerfile_content = fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

    // Check scripts directory
    if dockerfile_content.contains("COPY scripts") {
        assert!(
            Path::new("scripts").exists(),
            "Scripts directory referenced in Dockerfile does not exist"
        );
    }

    // Check installers directory
    if dockerfile_content.contains("COPY installers") {
        assert!(
            Path::new("installers").exists(),
            "Installers directory referenced in Dockerfile does not exist"
        );

        // Check that installers directory has expected structure
        let installer_dir = Path::new("installers");
        if installer_dir.exists() {
            let entries = fs::read_dir(installer_dir).expect("Failed to read installers directory");
            let mut has_platform_dirs = false;

            for entry in entries {
                let entry = entry.expect("Failed to read installer entry");
                if entry.path().is_dir() {
                    has_platform_dirs = true;
                    break;
                }
            }

            assert!(
                has_platform_dirs,
                "Installers directory exists but has no platform subdirectories"
            );
        }
    }
}
