use fukura::config::{FukuraConfig, RecordingConfig};
use std::fs;
use tempfile::TempDir;

/// Tests for configuration functionality related to time-based recording

#[test]
fn test_default_recording_config() {
    let config = RecordingConfig::default();
    assert_eq!(config.max_lookback_hours, 3);
    assert_eq!(config.min_lookback_minutes, 1);
}

#[test]
fn test_fukura_config_with_recording() {
    let config = FukuraConfig::default();
    assert_eq!(config.recording.max_lookback_hours, 3);
    assert_eq!(config.recording.min_lookback_minutes, 1);
}

#[test]
fn test_load_config_with_custom_recording_settings() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    let config_content = r#"
version = 1

[recording]
max_lookback_hours = 6
min_lookback_minutes = 5
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let loaded_config = FukuraConfig::load(&config_path).expect("Failed to load config");

    assert_eq!(loaded_config.recording.max_lookback_hours, 6);
    assert_eq!(loaded_config.recording.min_lookback_minutes, 5);
}

#[test]
fn test_load_config_with_partial_recording_settings() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    // Only specify max_lookback_hours, min should use default
    let config_content = r#"
version = 1

[recording]
max_lookback_hours = 12
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let loaded_config = FukuraConfig::load(&config_path).expect("Failed to load config");

    assert_eq!(loaded_config.recording.max_lookback_hours, 12);
    assert_eq!(loaded_config.recording.min_lookback_minutes, 1); // Default
}

#[test]
fn test_load_config_without_recording_section() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    // Config without recording section should use defaults
    let config_content = r#"
version = 1
profile = "test"
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let loaded_config = FukuraConfig::load(&config_path).expect("Failed to load config");

    // Should use defaults when section is missing
    assert_eq!(loaded_config.recording.max_lookback_hours, 3);
    assert_eq!(loaded_config.recording.min_lookback_minutes, 1);
}

#[test]
fn test_save_and_reload_config_with_recording() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    let mut config = FukuraConfig {
        version: 1,
        ..Default::default()
    };
    config.recording.max_lookback_hours = 8;
    config.recording.min_lookback_minutes = 10;

    // Save config
    config.save(&config_path).expect("Failed to save config");

    // Reload and verify
    let reloaded_config = FukuraConfig::load(&config_path).expect("Failed to reload config");

    assert_eq!(reloaded_config.version, 1);
    assert_eq!(reloaded_config.recording.max_lookback_hours, 8);
    assert_eq!(reloaded_config.recording.min_lookback_minutes, 10);
}

#[test]
fn test_config_validation_edge_cases() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    // Test with extreme values
    let config_content = r#"
version = 1

[recording]
max_lookback_hours = 168  # 1 week
min_lookback_minutes = 60 # 1 hour
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let loaded_config =
        FukuraConfig::load(&config_path).expect("Failed to load config with extreme values");

    assert_eq!(loaded_config.recording.max_lookback_hours, 168);
    assert_eq!(loaded_config.recording.min_lookback_minutes, 60);
}

#[test]
fn test_config_with_invalid_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    // Invalid TOML
    let invalid_config = r#"
version = 1
[recording
max_lookback_hours = "invalid"
"#;

    fs::write(&config_path, invalid_config).expect("Failed to write config");

    let result = FukuraConfig::load(&config_path);
    assert!(result.is_err(), "Should fail to load invalid TOML");
}

#[test]
fn test_config_with_wrong_types() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    // Wrong types for numeric values
    let config_content = r#"
version = 1

[recording]
max_lookback_hours = "not_a_number"
min_lookback_minutes = true
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let result = FukuraConfig::load(&config_path);
    assert!(
        result.is_err(),
        "Should fail to load config with wrong types"
    );
}

#[test]
fn test_config_nonexistent_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("nonexistent.toml");

    // Should return default config when file doesn't exist
    let config = FukuraConfig::load(&config_path)
        .expect("Should succeed with default config when file doesn't exist");

    // Should have default values
    assert_eq!(config.version, 0); // Default
    assert_eq!(config.recording.max_lookback_hours, 3);
    assert_eq!(config.recording.min_lookback_minutes, 1);
}

#[test]
fn test_global_config_fallback() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let local_config_path = temp_dir.path().join("local_config.toml");

    // Create local config with minimal settings
    let local_config_content = r#"
version = 1
"#;

    fs::write(&local_config_path, local_config_content).expect("Failed to write local config");

    // Test load_with_global_fallback (though we can't easily test global config in isolation)
    let config = FukuraConfig::load_with_global_fallback(&local_config_path)
        .expect("Should load with fallback");

    // Should have recording defaults even without explicit configuration
    assert_eq!(config.recording.max_lookback_hours, 3);
    assert_eq!(config.recording.min_lookback_minutes, 1);
}

#[test]
fn test_recording_config_serialization() {
    use serde_json;

    let config = RecordingConfig {
        max_lookback_hours: 24,
        min_lookback_minutes: 15,
    };

    // Test that it can be serialized and deserialized
    let serialized = serde_json::to_string(&config).expect("Should serialize recording config");

    let deserialized: RecordingConfig =
        serde_json::from_str(&serialized).expect("Should deserialize recording config");

    assert_eq!(deserialized.max_lookback_hours, 24);
    assert_eq!(deserialized.min_lookback_minutes, 15);
}

#[test]
fn test_reasonable_config_limits() {
    // Test that our default limits are reasonable
    let config = RecordingConfig::default();

    // Max should be reasonable (not too large to cause performance issues)
    assert!(
        config.max_lookback_hours <= 24,
        "Default max lookback should not exceed 24 hours"
    );
    assert!(
        config.max_lookback_hours >= 1,
        "Default max lookback should be at least 1 hour"
    );

    // Min should be reasonable (not too short to be useful)
    assert!(
        config.min_lookback_minutes >= 1,
        "Default min lookback should be at least 1 minute"
    );
    assert!(
        config.min_lookback_minutes <= 60,
        "Default min lookback should not exceed 1 hour"
    );

    // Min should be significantly less than max
    let min_in_hours = config.min_lookback_minutes as f64 / 60.0;
    let max_in_hours = config.max_lookback_hours as f64;
    assert!(
        min_in_hours < max_in_hours,
        "Minimum lookback should be less than maximum lookback"
    );
}
