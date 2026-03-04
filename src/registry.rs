use std::{collections::HashMap, fs, path::PathBuf};

use colored::Colorize;
use toml::Table;

use crate::{
    config::{ASSETS, config_dir},
    profile,
};

pub const REGISTRY_DIRNAME: &str = "registry";
const METADATA_FILENAME: &str = "metadata.toml";

fn registry_dir() -> PathBuf {
    config_dir().join(REGISTRY_DIRNAME)
}

fn copy_bundled_registry() -> Result<(), String> {
    let bundled_dir = ASSETS.get_dir(REGISTRY_DIRNAME).ok_or_else(|| {
        format!(
            "Bundled registry directory '{}' not found",
            REGISTRY_DIRNAME
        )
    })?;

    let target_dir = registry_dir();
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create registry directory: {}", e))?;

    for file in bundled_dir.files() {
        let filename = file
            .path()
            .file_name()
            .ok_or("Invalid bundled registry file path")?;
        let target_path = target_dir.join(filename);
        fs::write(&target_path, file.contents()).map_err(|e| {
            format!("Failed to write registry file {:?}: {}", filename, e)
        })?;
    }

    Ok(())
}

pub fn ensure_registry() -> Result<(), String> {
    let dir = registry_dir();
    let needs_init = !dir.exists()
        || fs::read_dir(&dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true);

    if needs_init {
        copy_bundled_registry()?;
    }

    Ok(())
}

pub fn read_registry() -> Result<toml::Value, String> {
    let dir = registry_dir();
    let mut packages = Table::new();

    let entries = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read registry directory: {}", e))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        let filename = match path.file_name().and_then(|f| f.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if !filename.ends_with(".toml") || filename == METADATA_FILENAME {
            continue;
        }

        let stem = filename.trim_end_matches(".toml");
        let content = fs::read_to_string(&path).map_err(|e| {
            format!("Failed to read registry file '{}': {}", filename, e)
        })?;
        let value: toml::Value = toml::from_str(&content).map_err(|e| {
            format!("Failed to parse registry file '{}': {}", filename, e)
        })?;

        packages.insert(stem.to_string(), value);
    }

    let mut root = Table::new();
    root.insert("package".to_string(), toml::Value::Table(packages));
    Ok(toml::Value::Table(root))
}

pub fn is_package_in_registry(package_name: &str) -> Result<bool, String> {
    let registry = read_registry()?;
    let packages = registry
        .get("package")
        .and_then(|p| p.as_table())
        .ok_or("Registry is missing the '[package]' table")?;

    if packages.contains_key(package_name) {
        return Ok(true);
    }

    copy_bundled_registry()?;
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
    println!("{}", "Available packages:".bold());

    for (name, details) in packages_table {
        if let Some(q) = query
            && !name.to_lowercase().contains(&q.to_lowercase())
        {
            continue;
        }
        found = true;

        println!("- {}", name.cyan().bold());
        if let Some(installers) =
            details.get("packages").and_then(|i| i.as_table())
        {
            println!("  {}", "Installers:".dimmed());
            for (installer_name, value) in installers {
                if installer_name == "custom" {
                    if let Some(cmd) = value.as_str() {
                        println!(
                            "    - {}: {}",
                            "custom".yellow(),
                            cmd.dimmed()
                        );
                    }
                } else {
                    println!("    - {}", installer_name.green());
                }
            }
        } else {
            println!("  {}", "No installers specified.".dimmed());
        }
    }

    if !found {
        println!("{}", "No packages found matching your query.".yellow());
    }

    Ok(())
}

fn get_raw_package_table(package_name: &str) -> Result<Table, String> {
    let lookup = |registry: &toml::Value| -> Option<Table> {
        registry
            .get("package")
            .and_then(|v| v.as_table())
            .and_then(|pkgs| pkgs.get(package_name))
            .and_then(|v| v.as_table())
            .cloned()
    };

    let registry = read_registry()?;
    if let Some(table) = lookup(&registry) {
        return Ok(table);
    }

    copy_bundled_registry()?;
    let registry = read_registry()?;
    lookup(&registry).ok_or_else(|| {
        format!("Package '{}' not found in registry", package_name)
    })
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

    Ok(Vec::new())
}

pub fn update_registry() -> Result<(), String> {
    let base_url = crate::config::get_registry_url();
    let metadata_url = format!("{}/{}", base_url, METADATA_FILENAME);

    let remote_body = ureq::get(&metadata_url)
        .call()
        .map_err(|e| format!("Failed to fetch registry metadata: {}", e))?
        .into_string()
        .map_err(|e| {
            format!("Failed to read registry metadata response: {}", e)
        })?;

    let remote_meta: toml::Value = toml::from_str(&remote_body)
        .map_err(|e| format!("Failed to parse remote metadata: {}", e))?;

    let remote_version = remote_meta
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("Remote metadata missing 'version' field")?;

    let remote_packages: Vec<String> = remote_meta
        .get("packages")
        .and_then(|v| v.as_array())
        .ok_or("Remote metadata missing 'packages' array")?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    let local_metadata_path = registry_dir().join(METADATA_FILENAME);
    if local_metadata_path.exists() {
        let local_body = fs::read_to_string(&local_metadata_path)
            .map_err(|e| format!("Failed to read local metadata: {}", e))?;
        if let Ok(local_meta) = toml::from_str::<toml::Value>(&local_body)
            && local_meta.get("version").and_then(|v| v.as_str())
                == Some(remote_version)
        {
            println!(
                "{} (version {}).",
                "Registry is already up to date".green(),
                remote_version.bold()
            );
            return Ok(());
        }
    }

    let dir = registry_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create registry directory: {}", e))?;

    for pkg_name in &remote_packages {
        let pkg_url = format!("{}/{}.toml", base_url, pkg_name);
        match ureq::get(&pkg_url).call() {
            Ok(response) => {
                let content = response.into_string().map_err(|e| {
                    format!("Failed to read response for '{}': {}", pkg_name, e)
                })?;
                let dest = dir.join(format!("{}.toml", pkg_name));
                fs::write(&dest, &content).map_err(|e| {
                    format!("Failed to write package '{}': {}", pkg_name, e)
                })?;
            }
            Err(e) => {
                eprintln!(
                    "{} failed to fetch package '{}': {}",
                    "Warning:".yellow().bold(),
                    pkg_name.cyan(),
                    e
                );
            }
        }
    }

    fs::write(&local_metadata_path, &remote_body)
        .map_err(|e| format!("Failed to write metadata: {}", e))?;

    println!(
        "{} version {} ({} packages).",
        "Registry updated to".green(),
        remote_version.bold(),
        remote_packages.len()
    );
    Ok(())
}

pub fn add_custom_package(file: &str) -> Result<(), String> {
    let content = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file '{}': {}", file, e))?;

    toml::from_str::<toml::Value>(&content)
        .map_err(|e| format!("Invalid package file: {}", e))?;

    let filename = PathBuf::from(file)
        .file_name()
        .ok_or("Invalid file path")?
        .to_os_string();

    let dest = registry_dir().join(&filename);
    fs::copy(file, &dest)
        .map_err(|e| format!("Failed to copy package file: {}", e))?;

    println!(
        "{} from '{}'.",
        "Package added to registry".green(),
        file.cyan()
    );
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

    fn create_dummy_registry(_temp_dir: &TempDir, packages: &[(&str, &str)]) {
        let reg_dir = crate::config::config_dir().join(REGISTRY_DIRNAME);
        fs::create_dir_all(&reg_dir).expect("Failed to create registry dir");

        fs::write(reg_dir.join(METADATA_FILENAME), "version = \"2\"\n")
            .expect("Failed to write metadata");

        for (name, content) in packages {
            fs::write(reg_dir.join(format!("{}.toml", name)), content)
                .expect("Failed to write package file");
        }
    }

    #[test]
    #[serial]
    fn test_ensure_registry_creates_dir_if_missing() {
        let _temp = setup_test_env();
        let dir = registry_dir();

        assert!(!dir.exists());

        let result = ensure_registry();
        assert!(result.is_ok());
        assert!(dir.exists());

        let entries: Vec<_> =
            fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).collect();
        assert!(entries.len() > 1);
    }

    #[test]
    #[serial]
    fn test_ensure_registry_skips_if_populated() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[("mypkg", "display = \"My Package\"\n")],
        );

        ensure_registry().unwrap();

        let reg_dir = registry_dir();
        assert!(reg_dir.join("mypkg.toml").exists());
        assert!(!reg_dir.join("curl.toml").exists());
    }

    #[test]
    #[serial]
    fn test_read_registry_merges_files() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[
                (
                    "curl",
                    r#"display = "cURL"
detect = "curl --version"

[packages]
apt = "curl"
brew = "curl"
"#,
                ),
                (
                    "git",
                    r#"display = "Git"
detect = "git --version"

[packages]
apt = "git"
"#,
                ),
            ],
        );

        let registry = read_registry().unwrap();
        let packages =
            registry.get("package").and_then(|p| p.as_table()).unwrap();
        assert!(packages.contains_key("curl"));
        assert!(packages.contains_key("git"));
        assert!(!packages.contains_key("metadata"));
    }

    #[test]
    #[serial]
    fn test_get_package_details_full() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[
                (
                    "curl",
                    r#"display = "cURL"
detect = "curl --version"
dependencies = ["git"]

[packages]
apt = "curl"
brew = "curl"
"#,
                ),
                (
                    "git",
                    r#"display = "Git"
detect = "git --version"

[packages]
apt = "git"
"#,
                ),
            ],
        );

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
        create_dummy_registry(
            &_temp,
            &[("existing", "display = \"Existing Package\"\n")],
        );

        let result = get_package_details("non_existent");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("not found in registry"));
    }

    #[test]
    #[serial]
    fn test_list_packages_no_query_runs() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[
                (
                    "curl",
                    r#"display = "cURL"
[packages]
apt = "curl"
custom = "install-curl.sh"
"#,
                ),
                (
                    "git",
                    r#"display = "Git"
[packages]
brew = "git"
"#,
                ),
            ],
        );

        let result = list_packages(&None);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_list_packages_with_query_runs() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[
                (
                    "curl",
                    r#"display = "cURL"
[packages]
apt = "curl"
"#,
                ),
                (
                    "git",
                    r#"display = "Git"
[packages]
brew = "git"
"#,
                ),
            ],
        );

        let query = Some("git".to_string());
        let result = list_packages(&query);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_list_packages_no_match_runs() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[(
                "curl",
                r#"display = "cURL"
[packages]
apt = "curl"
"#,
            )],
        );

        let query = Some("nonexistent".to_string());
        let result = list_packages(&query);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_is_package_in_registry_exists() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[(
                "git",
                r#"display = "Git"
[packages]
apt = "git"
"#,
            )],
        );

        let result = is_package_in_registry("git");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    #[serial]
    fn test_is_package_in_registry_not_exists() {
        let _temp = setup_test_env();
        create_dummy_registry(&_temp, &[("git", "display = \"Git\"\n")]);

        let result = is_package_in_registry("nonexistent");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    #[serial]
    fn test_is_package_in_registry_empty_registry() {
        let _temp = setup_test_env();
        let reg_dir = registry_dir();
        fs::create_dir_all(&reg_dir).unwrap();
        fs::write(reg_dir.join(METADATA_FILENAME), "version = \"2\"\n")
            .unwrap();

        let result = is_package_in_registry("curl");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    #[serial]
    fn test_get_dependencies_with_deps() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[
                (
                    "curl",
                    r#"display = "cURL"
dependencies = ["git", "openssl"]

[packages]
apt = "curl"
"#,
                ),
                ("git", "display = \"Git\"\n"),
            ],
        );

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
        create_dummy_registry(
            &_temp,
            &[(
                "git",
                r#"display = "Git"
[packages]
apt = "git"
"#,
            )],
        );

        let result = get_dependencies("git");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    #[serial]
    fn test_get_dependencies_non_existent() {
        let _temp = setup_test_env();
        create_dummy_registry(&_temp, &[("git", "display = \"Git\"\n")]);

        let result = get_dependencies("nonexistent");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("not found"));
    }

    #[test]
    #[serial]
    fn test_read_registry_success() {
        let _temp = setup_test_env();
        create_dummy_registry(&_temp, &[("test", "display = \"Test\"\n")]);

        let result = read_registry();
        assert!(result.is_ok());

        let registry = result.unwrap();
        assert!(registry.get("package").is_some());
    }

    #[test]
    #[serial]
    fn test_add_custom_package_valid() {
        let _temp = setup_test_env();
        create_dummy_registry(&_temp, &[]);

        let pkg_file = _temp.path().join("mypkg.toml");
        fs::write(&pkg_file, "display = \"My Package\"\n").unwrap();

        let result = add_custom_package(pkg_file.to_str().unwrap());
        assert!(result.is_ok());
        assert!(registry_dir().join("mypkg.toml").exists());
    }

    #[test]
    #[serial]
    fn test_add_custom_package_invalid_toml() {
        let _temp = setup_test_env();
        create_dummy_registry(&_temp, &[]);

        let pkg_file = _temp.path().join("bad.toml");
        fs::write(&pkg_file, "not valid ][[[").unwrap();

        let result = add_custom_package(pkg_file.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Invalid package file"));
    }

    #[test]
    #[serial]
    fn test_add_custom_package_missing_file() {
        let _temp = setup_test_env();
        create_dummy_registry(&_temp, &[]);

        let result = add_custom_package("/nonexistent/path/pkg.toml");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Failed to read file"));
    }

    #[test]
    #[serial]
    fn test_lazy_refresh_on_package_miss() {
        let _temp = setup_test_env();
        create_dummy_registry(
            &_temp,
            &[("mypkg", "display = \"My Package\"\n")],
        );

        assert!(!registry_dir().join("curl.toml").exists());

        let result = is_package_in_registry("curl");
        assert!(result.is_ok());
        assert!(result.unwrap());
        assert!(registry_dir().join("curl.toml").exists());
    }
}
