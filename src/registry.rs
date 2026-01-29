use std::{collections::HashMap, fs};

use toml::Table;

use crate::{
    config::{ASSETS, config_dir},
    profile,
}; // Import profile module to use ProfilePackage

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

pub fn read_registry() -> Result<toml::Value, String> {
    let registry_path = config_dir().join(REGISTRY_FILENAME);
    let content = fs::read_to_string(&registry_path)
        .map_err(|e| format!("Failed to read registry file: {}", e))?;
    let value: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse registry TOML: {}", e))?;
    Ok(value)
}

pub fn is_package_in_registry(package_name: &str) -> Result<bool, String> {
    let registry = read_registry()?;
    let packages = registry
        .get("package")
        .and_then(|p| p.as_table())
        .ok_or("Registry is missing the '[package]' table")?;

    Ok(packages.contains_key(package_name))
}

pub fn list_packages(query: &Option<String>) -> Result<(), String> {
    let registry = read_registry()?;
    let packages_table = registry
        .get("package")
        .and_then(|p| p.as_table())
        .ok_or("Registry is missing the '[package]' table")?;

    let mut found = false;
    println!("Available packages:");

    for (name, details) in packages_table {
        if let Some(q) = query
            && !name.to_lowercase().contains(&q.to_lowercase())
        {
            continue;
        }
        found = true;

        println!("- {}", name);
        if let Some(installers) =
            details.get("packages").and_then(|i| i.as_table())
        {
            println!("  Installers:");
            for (installer_name, value) in installers {
                if installer_name == "custom" {
                    if let Some(cmd) = value.as_str() {
                        println!("    - custom: {}", cmd);
                    }
                } else {
                    println!("    - {}", installer_name);
                }
            }
        } else {
            println!("  No installers specified.");
        }
    }

    if !found {
        println!("No packages found matching your query.");
    }

    Ok(())
}

fn get_raw_package_table(package_name: &str) -> Result<Table, String> {
    let registry = read_registry()?;
    let packages_table = registry
        .get("package")
        .and_then(|v| v.as_table())
        .ok_or_else(|| {
            "Registry is missing the '[package]' table".to_string()
        })?;

    let package_table = packages_table
        .get(package_name)
        .and_then(|v| v.as_table())
        .ok_or_else(|| {
            format!("Package '{}' not found in registry", package_name)
        })?;

    Ok(package_table.clone())
}

pub fn get_package_details(
    package_name: &str,
) -> Result<profile::ProfilePackage, String> {
    let package_table = get_raw_package_table(package_name)?;

    let display = package_table
        .get("display")
        .and_then(|v| v.as_str())
        .map(String::from);
    let detect = package_table
        .get("detect")
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut installers = HashMap::new();
    if let Some(packages_section) =
        package_table.get("packages").and_then(|v| v.as_table())
    {
        for (key, value) in packages_section {
            if let Some(v_str) = value.as_str() {
                installers.insert(key.clone(), v_str.to_string());
            }
        }
    }

    let dependencies = get_dependencies(package_name)?;

    Ok(profile::ProfilePackage {
        name: package_name.to_string(),
        display,
        installers,
        detect,
        dependencies,
    })
}

pub fn get_dependencies(package_name: &str) -> Result<Vec<String>, String> {
    let registry = read_registry()?;
    let packages_table = registry
        .get("package")
        .and_then(|v| v.as_table())
        .ok_or_else(|| {
            "Registry is missing the '[package]' table".to_string()
        })?;

    let package_table = packages_table
        .get(package_name)
        .and_then(|v| v.as_table())
        .ok_or_else(|| {
            format!("Package '{}' not found in registry", package_name)
        })?;

    if let Some(deps_value) = package_table.get("dependencies") {
        let deps_array = deps_value.as_array().ok_or_else(|| {
            format!(
                "'dependencies' field for '{}' is not an array",
                package_name
            )
        })?;

        let deps = deps_array
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        return Ok(deps);
    }

    Ok(Vec::new()) // No dependencies
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

    // Helper to create a dummy registry.toml for testing
    fn create_dummy_registry(_temp_dir: &TempDir, content: &str) {
        let registry_path = crate::config::config_dir().join(REGISTRY_FILENAME);
        fs::write(&registry_path, content)
            .expect("Failed to write dummy registry");
    }

    #[test]
    #[serial]
    fn test_get_package_details_full() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.curl]
            display = "cURL"
            packages.apt = "curl"
            packages.brew = "curl"
            detect = "curl --version"
            dependencies = ["git"]

            [package.git]
            display = "Git"
            packages.apt = "git"
            detect = "git --version"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let details = get_package_details("curl").unwrap();
        assert_eq!(details.name, "curl");
        assert_eq!(details.display, Some("cURL".to_string()));
        assert_eq!(details.detect, Some("curl --version".to_string()));
        assert_eq!(details.installers.get("apt"), Some(&"curl".to_string()));
        assert_eq!(details.installers.get("brew"), Some(&"curl".to_string()));
        assert_eq!(details.dependencies, vec!["git".to_string()]);

        let details_git = get_package_details("git").unwrap();
        assert_eq!(details_git.name, "git");
        assert_eq!(details_git.display, Some("Git".to_string()));
        assert_eq!(details_git.installers.get("apt"), Some(&"git".to_string()));
        assert!(!details_git.installers.contains_key("brew"));
        assert!(details_git.dependencies.is_empty());
    }

    #[test]
    #[serial]
    fn test_get_package_details_non_existent() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.existing]
            display = "Existing Package"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = get_package_details("non_existent");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("not found in registry"));
    }

    #[test]
    #[serial]
    fn test_list_packages_no_query_runs() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.curl]
            display = "cURL"
            packages.apt = "curl"
            packages.custom = "install-curl.sh"

            [package.git]
            display = "Git"
            packages.brew = "git"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = list_packages(&None);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_list_packages_with_query_runs() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.curl]
            display = "cURL"
            packages.apt = "curl"

            [package.git]
            display = "Git"
            packages.brew = "git"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let query = Some("git".to_string());
        let result = list_packages(&query);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_list_packages_no_match_runs() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.curl]
            display = "cURL"
            packages.apt = "curl"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let query = Some("nonexistent".to_string());
        let result = list_packages(&query);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_is_package_in_registry_exists() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.git]
            display = "Git"
            packages.apt = "git"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = is_package_in_registry("git");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    #[serial]
    fn test_is_package_in_registry_not_exists() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.git]
            display = "Git"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = is_package_in_registry("nonexistent");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    #[serial]
    fn test_is_package_in_registry_missing_package_section() {
        let _temp = setup_test_env();
        let dummy_registry = r#"version = "1""#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = is_package_in_registry("anything");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("missing"));
    }

    #[test]
    #[serial]
    fn test_get_dependencies_with_deps() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.curl]
            display = "cURL"
            packages.apt = "curl"
            dependencies = ["git", "openssl"]

            [package.git]
            display = "Git"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = get_dependencies("curl");
        assert!(result.is_ok());

        let deps = result.unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"git".to_string()));
        assert!(deps.contains(&"openssl".to_string()));
    }

    #[test]
    #[serial]
    fn test_get_dependencies_no_deps() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.git]
            display = "Git"
            packages.apt = "git"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = get_dependencies("git");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    #[serial]
    fn test_get_dependencies_non_existent() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.git]
            display = "Git"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = get_dependencies("nonexistent");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("not found"));
    }

    #[test]
    #[serial]
    fn test_read_registry_success() {
        let _temp = setup_test_env();
        let dummy_registry = r#"
            version = "1"

            [package.test]
            display = "Test"
        "#;
        create_dummy_registry(&_temp, dummy_registry);

        let result = read_registry();
        assert!(result.is_ok());

        let registry = result.unwrap();
        assert!(registry.get("version").is_some());
        assert!(registry.get("package").is_some());
    }
}
