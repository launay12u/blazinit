use std::fs;

use crate::config::{ASSETS, config_dir};

const REGISTRY_FILENAME: &str = "registry.toml";

fn get_bundled_registry_content() -> Result<&'static [u8], String> {
    let file = ASSETS.get_file(REGISTRY_FILENAME).ok_or_else(|| {
        format!("Bundled registry '{}' not found", REGISTRY_FILENAME)
    })?;

    Ok(file.contents())
}

fn copy_bundled_registry() -> Result<(), String> {
    let target = config_dir().join("registry.toml");
    let content = get_bundled_registry_content()?;
    fs::write(target, content)
        .map_err(|e| format!("Failed to write registry: {}", e))?;
    Ok(())
}

fn get_bundled_registry_version() -> Result<String, String> {
    let content = get_bundled_registry_content()?;

    let str_content = std::str::from_utf8(content)
        .map_err(|e| format!("Invalid UTF-8 in bundled registry: {}", e))?;
    let value: toml::Value = toml::from_str(str_content)
        .map_err(|e| format!("Failed to parse TOML: {}", e))?;

    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("Bundled registry missing 'version' field")?;

    Ok(version.to_string())
}

fn get_current_registry_version() -> Result<String, String> {
    let current_path = config_dir().join(REGISTRY_FILENAME);

    let content = fs::read(&current_path)
        .map_err(|e| format!("Failed to read current registry: {}", e))?;

    let str_content = std::str::from_utf8(&content)
        .map_err(|e| format!("Invalid UTF-8 in current registry: {}", e))?;

    let value: toml::Value = toml::from_str(str_content)
        .map_err(|e| format!("Failed to parse TOML: {}", e))?;

    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("Current registry missing 'version' field")?;

    Ok(version.to_string())
}

pub fn update_registry_version_if_needed() -> Result<(), String> {
    let bundled_version = get_bundled_registry_version()?;

    let need_update = match get_current_registry_version() {
        Ok(current_version) => current_version != bundled_version,
        Err(_) => true,
    };

    if need_update {
        copy_bundled_registry()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        unsafe {
            env::set_var("HOME", temp_dir.path());
            env::set_var("XDG_CONFIG_HOME", temp_dir.path().join(".config"));
        }

        let config_path = crate::config::config_dir();
        fs::create_dir_all(&config_path).expect("Failed to create config path");

        temp_dir
    }

    #[test]
    #[serial]
    fn test_registry_created_if_missing() {
        let _temp = setup_test_env();
        let target = crate::config::config_dir().join(REGISTRY_FILENAME);

        assert!(!target.exists());

        let result = update_registry_version_if_needed();
        assert!(result.is_ok());

        assert!(target.exists());
        let content = fs::read_to_string(target).unwrap();
        assert!(content.contains("version"));
    }

    #[test]
    #[serial]
    fn test_registry_updated_if_version_mismatch() {
        let _temp = setup_test_env();
        let target = crate::config::config_dir().join(REGISTRY_FILENAME);

        // Create an old version
        let old_registry = r#"
            version = "0.0.0"
            [packages]
            fake = "old"
        "#;
        fs::write(&target, old_registry).unwrap();

        let result = update_registry_version_if_needed();
        assert!(result.is_ok());

        let content = fs::read_to_string(target).unwrap();
        // Should not be the old one
        assert!(!content.contains("fake = \"old\""));
        // Should be the bundled one
        assert!(content.contains("version"));
    }

    #[test]
    #[serial]
    fn test_registry_not_updated_if_version_matches() {
        let _temp = setup_test_env();
        let target = crate::config::config_dir().join(REGISTRY_FILENAME);

        // 1. Populate with current version
        update_registry_version_if_needed().unwrap();
        let initial_content = fs::read_to_string(&target).unwrap();

        // 2. Modify content but keep the same version
        let modified_content =
            format!("{}\n# Modified by test", initial_content);
        fs::write(&target, &modified_content).unwrap();

        // 3. Run update again
        update_registry_version_if_needed().unwrap();

        // 4. Check that file was NOT overwritten
        let final_content = fs::read_to_string(&target).unwrap();
        assert!(final_content.contains("# Modified by test"));
    }
}
