use std::{fs, path::PathBuf};

use colored::Colorize;
use dirs_next;
use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};

use crate::{
    profile::{PROFILE_DIRNAME, ensure_default_profile},
    registry::ensure_registry,
};

pub static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

const CONFIG_FILENAME: &str = "config.toml";
const DEFAULT_PROFILE_NAME: &str = "default";
const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/launay12u/blazinit/main/assets/registry";

#[derive(Serialize, Deserialize)]
struct Config {
    default_profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    preferred_installer: Option<String>,
    #[serde(default = "default_registry_url")]
    registry_url: String,
}

fn default_registry_url() -> String {
    DEFAULT_REGISTRY_URL.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_profile: DEFAULT_PROFILE_NAME.to_string(),
            preferred_installer: None,
            registry_url: DEFAULT_REGISTRY_URL.to_string(),
        }
    }
}

pub fn get_preferred_installer() -> Option<String> {
    let val = read_config().preferred_installer;
    log::debug!("preferred_installer={:?}", val);
    val
}

pub fn get_registry_url() -> String {
    let url = read_config().registry_url;
    log::debug!("registry_url={}", url);
    url
}

fn config_file_path() -> PathBuf {
    config_dir().join(CONFIG_FILENAME)
}

fn read_config() -> Config {
    let path = config_file_path();
    if !path.exists() {
        log::debug!("config file not found at {:?}, using defaults", path);
        return Config::default();
    }
    log::debug!("reading config from {:?}", path);
    let content = fs::read_to_string(path).unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
}

pub fn get_default_profile() -> String {
    read_config().default_profile
}

pub fn set_default_profile(profile_name: &str) -> Result<(), String> {
    let profile_path = crate::profile::profile_path(profile_name);
    if !profile_path.exists() {
        log::error!("set_default_profile: profile '{}' does not exist", profile_name);
        return Err(format!("Profile '{}' does not exist.", profile_name));
    }

    let mut config = read_config();
    config.default_profile = profile_name.to_string();

    let toml_str = toml::to_string(&config).map_err(|e| e.to_string())?;
    fs::write(config_file_path(), toml_str).map_err(|e| e.to_string())?;

    log::info!("default profile set to '{}'", profile_name);
    println!(
        "{} '{}'.",
        "Default profile set to".green(),
        profile_name.cyan()
    );

    Ok(())
}

pub fn config_dir() -> PathBuf {
    dirs_next::config_dir()
        .expect("Cannot find config directory")
        .join("blazinit")
}

pub fn profiles_dir() -> PathBuf {
    let dir = config_dir().join(PROFILE_DIRNAME);
    fs::create_dir_all(&dir).expect("Failed to create profiles directory");
    dir
}

pub fn bootstrap_config() -> Result<(), String> {
    let base = config_dir();
    log::debug!("bootstrap: config dir = {:?}", base);
    fs::create_dir_all(&base)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    let default = get_default_profile();
    log::debug!("bootstrap: ensuring default profile '{}'", default);
    ensure_default_profile(&default)?;

    log::debug!("bootstrap: ensuring registry");
    ensure_registry()?;

    log::debug!("bootstrap: checking for silent registry update");
    crate::registry::try_update_registry_silent();

    log::debug!("bootstrap: complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;

    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    // Helper to set up a temporary environment for testing
    // Returns the TempDir so it isn't dropped until the test ends
    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        unsafe {
            env::set_var("HOME", temp_dir.path());
            // Also set XDG_CONFIG_HOME for Linux environments to be sure
            env::set_var("XDG_CONFIG_HOME", temp_dir.path().join(".config"));
        }
        // Ensure config directory exists
        fs::create_dir_all(config_dir()).expect("Failed to create config dir");
        temp_dir
    }

    // Helper to create a dummy profile for testing set_default_profile
    fn create_dummy_profile(
        _temp_dir: &TempDir,
        profile_name: &str,
    ) -> Result<(), String> {
        // Use the actual config functions to get the correct path
        let profile_path = crate::profile::profile_path(profile_name);

        fs::create_dir_all(profile_path.parent().unwrap())
            .expect("Failed to create profiles dir");

        // Use an empty packages list in the dummy profile
        let content = format!("name = \"{}\"\npackages = []", profile_name);
        fs::write(&profile_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_default_profile_initial() {
        let _temp = setup_test_env();
        assert_eq!(get_default_profile(), "default");
    }

    #[test]
    #[serial]
    fn test_set_default_profile_success() {
        let temp = setup_test_env();
        let profile_name = "my_new_default";
        create_dummy_profile(&temp, profile_name).unwrap();

        let result = set_default_profile(profile_name);
        assert!(result.is_ok());
        assert_eq!(get_default_profile(), profile_name);

        let config_content = fs::read_to_string(config_file_path()).unwrap();
        assert!(
            config_content
                .contains(&format!("default_profile = \"{}\"", profile_name))
        );
    }

    #[test]
    #[serial]
    fn test_set_default_profile_non_existent() {
        let _temp = setup_test_env();
        let profile_name = "non_existent";
        let result = set_default_profile(profile_name);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            format!("Profile '{}' does not exist.", profile_name)
        );
        assert_eq!(get_default_profile(), "default"); // Should remain default
    }

    #[test]
    #[serial]
    fn test_get_default_profile_after_set() {
        let temp = setup_test_env();
        let profile_name = "another_default";
        create_dummy_profile(&temp, profile_name).unwrap();
        set_default_profile(profile_name).unwrap();

        assert_eq!(get_default_profile(), profile_name);
    }
}
