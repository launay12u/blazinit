use std::{fs, path::PathBuf};

use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::config::profiles_dir;

#[derive(Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub software: Vec<String>,
}

pub const DEFAULT_PROFILE: &str = "default";
pub const PROFILE_DIRNAME: &str = "profiles";

pub fn ensure_default_profile() -> Result<(), String> {
    let path = profile_path(DEFAULT_PROFILE);
    if !path.exists() {
        let profile = Profile {
            name: DEFAULT_PROFILE.to_string(),
            software: Vec::new(),
        };
        let toml_str = toml::to_string(&profile).map_err(|e| e.to_string())?;
        fs::write(path, toml_str).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn profile_path(profile_name: &str) -> PathBuf {
    let mut path = profiles_dir();
    path.push(format!("{}.toml", profile_name));
    path
}

pub fn create_profile(profile_name: &str) -> Result<(), String> {
    let path = profile_path(profile_name);

    if path.exists() {
        return Err(format!("Profile '{}' already exists", profile_name));
    }

    let profile = Profile {
        name: profile_name.to_string(),
        software: Vec::new(),
    };

    let toml_str = toml::to_string(&profile).map_err(|e| e.to_string())?;
    fs::write(path, toml_str).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_profile(profile_name: &str) -> Result<(), String> {
    if profile_name == DEFAULT_PROFILE {
        return Err(format!(
            "Cannot delete the default profile '{}'",
            DEFAULT_PROFILE
        ));
    }

    let path = profile_path(profile_name);

    if !path.exists() {
        return Err(format!("Profile '{}' does not exist", profile_name));
    }

    fs::remove_file(path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_profiles() {
    let mut out = std::io::stdout();
    let _ = list_profiles_to(&mut out);
}

pub fn list_profiles_to<W: std::io::Write>(
    writer: &mut W,
) -> std::io::Result<()> {
    let path = profiles_dir();

    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(_) => {
            writeln!(writer, "No profiles found.")?;
            return Ok(());
        }
    };

    writeln!(writer, "Saved profiles:")?;

    for entry in entries {
        if let Ok(entry) = entry
            && let Some(name) =
                entry.path().file_stem().and_then(|s| s.to_str())
        {
            if name == DEFAULT_PROFILE {
                writeln!(writer, "{} (default)", name.green())?;
            } else {
                writeln!(writer, "{}", name)?;
            }
        }
    }
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
        temp_dir
    }

    #[test]
    #[serial]
    fn test_ensure_default_profile_creates_file() {
        let _temp = setup_test_env();

        let result = ensure_default_profile();
        assert!(result.is_ok());

        let path = profile_path(DEFAULT_PROFILE);
        assert!(path.exists());

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains(&format!("name = \"{}\"", DEFAULT_PROFILE)));
    }

    #[test]
    #[serial]
    fn test_create_profile_success() {
        let _temp = setup_test_env();

        let profile_name = "my-profile";
        let result = create_profile(profile_name);
        assert!(result.is_ok());

        let path = profile_path(profile_name);
        assert!(path.exists());

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains(&format!("name = \"{}\"", profile_name)));
    }

    #[test]
    #[serial]
    fn test_create_profile_already_exists() {
        let _temp = setup_test_env();
        let profile_name = "duplicate";

        create_profile(profile_name).unwrap();
        let result = create_profile(profile_name);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            format!("Profile '{}' already exists", profile_name)
        );
    }

    #[test]
    #[serial]
    fn test_delete_profile_success() {
        let _temp = setup_test_env();
        let profile_name = "to-delete";

        create_profile(profile_name).unwrap();
        assert!(profile_path(profile_name).exists());

        let result = delete_profile(profile_name);
        assert!(result.is_ok());
        assert!(!profile_path(profile_name).exists());
    }

    #[test]
    #[serial]
    fn test_delete_default_profile_fails() {
        let _temp = setup_test_env();
        ensure_default_profile().unwrap();

        let result = delete_profile(DEFAULT_PROFILE);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .contains("Cannot delete the default profile")
        );

        // Ensure it still exists
        assert!(profile_path(DEFAULT_PROFILE).exists());
    }

    #[test]
    #[serial]
    fn test_delete_non_existent_profile() {
        let _temp = setup_test_env();
        let result = delete_profile("ghost");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("does not exist"));
    }

    #[test]
    #[serial]
    fn test_list_profiles_output() {
        let _temp = setup_test_env();

        ensure_default_profile().unwrap();
        create_profile("another").unwrap();

        let mut output = Vec::new();
        list_profiles_to(&mut output).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("Saved profiles:"));
        assert!(output.contains("default"));
        assert!(output.contains("another"));
    }
}
