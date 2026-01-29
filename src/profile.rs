use std::{collections::HashMap, fs, path::PathBuf};

use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::{config, config::profiles_dir};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfilePackage {
    pub name: String,
    pub display: Option<String>,
    #[serde(default)]
    pub installers: HashMap<String, String>,
    pub detect: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Profile {
    pub name: String,
    pub packages: Vec<ProfilePackage>,
}

pub const PROFILE_DIRNAME: &str = "profiles";

pub fn ensure_default_profile(profile_name: &str) -> Result<(), String> {
    let path = profile_path(profile_name);
    if !path.exists() {
        let profile = Profile {
            name: profile_name.to_string(),
            packages: Vec::new(),
        };
        let toml_str = toml::to_string(&profile).map_err(|e| e.to_string())?;
        fs::write(path, toml_str).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn profile_path(profile_name: &str) -> PathBuf {
    let mut path = profiles_dir();
    path.push(format!("{}.toml", profile_name));
    path
}

pub fn read_profile(profile_name: &str) -> Result<Profile, String> {
    let path = profile_path(profile_name);
    if !path.exists() {
        return Err(format!("Profile '{}' does not exist.", profile_name));
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let profile: Profile =
        toml::from_str(&content).map_err(|e| e.to_string())?;
    Ok(profile)
}

pub fn write_profile(profile: &Profile) -> Result<(), String> {
    let path = profile_path(&profile.name);
    let toml_str = toml::to_string(profile).map_err(|e| e.to_string())?;
    fs::write(path, toml_str).map_err(|e| e.to_string())
}

pub fn add_package_to_profile(
    profile_name: &str,
    package_name: &str,
) -> Result<(), String> {
    let mut profile = read_profile(profile_name)?;

    let mut to_add_name_queue = vec![package_name.to_string()];
    let mut processed_names = std::collections::HashSet::new();
    let mut new_additions_details: Vec<ProfilePackage> = Vec::new();

    let existing_package_names: std::collections::HashSet<_> =
        profile.packages.iter().map(|p| p.name.clone()).collect();

    while let Some(current_package_name) = to_add_name_queue.pop() {
        if existing_package_names.contains(&current_package_name)
            || processed_names.contains(&current_package_name)
        {
            continue;
        }

        let package_details = match crate::registry::get_package_details(
            &current_package_name,
        ) {
            Ok(details) => details,
            Err(e) => {
                eprintln!(
                    "Warning: Package '{}' (dependency) not found in registry. Skipping. Error: {}",
                    current_package_name, e
                );
                continue;
            }
        };

        new_additions_details.push(package_details.clone());
        processed_names.insert(current_package_name.clone());

        for dep_name in package_details.dependencies {
            to_add_name_queue.push(dep_name);
        }
    }

    if !new_additions_details.is_empty() {
        println!("Adding to profile '{}':", profile_name);
        for item in &new_additions_details {
            println!("- {}", item.name);
            profile.packages.push(item.clone());
        }

        profile.packages.sort_by(|a, b| a.name.cmp(&b.name));

        write_profile(&profile)?;
        println!(
            "Successfully added {} package(s).",
            new_additions_details.len()
        );
    } else {
        println!(
            "Package '{}' and its dependencies are already in profile '{}'.",
            package_name, profile_name
        );
    }

    Ok(())
}

pub fn show_profile(profile_name: &str) -> Result<(), String> {
    let p = read_profile(profile_name)
        .map_err(|e| format!("Failed to read profile: {}", e))?;
    println!("Profile: {}", p.name);
    if p.packages.is_empty() {
        println!("  No packages in this profile.");
    } else {
        println!("  Packages:");
        for pkg in p.packages {
            let display_name = pkg.display.unwrap_or_else(|| pkg.name.clone());
            println!("  - {}", display_name);
            if let Some(detect) = pkg.detect {
                println!("    Detect: {}", detect);
            }
            if !pkg.installers.is_empty() {
                println!("    Installers:");
                for (name, command) in pkg.installers {
                    println!("      - {}: {}", name, command);
                }
            }
            if !pkg.dependencies.is_empty() {
                println!("    Dependencies: {:?}", pkg.dependencies);
            }
        }
    }
    Ok(())
}

pub fn remove_package_from_profile(
    profile_name: &str,
    package_name: &str,
) -> Result<(), String> {
    println!(
        "Remove package '{}' from profile '{}'",
        package_name, profile_name
    );
    Ok(())
}

pub fn export_profile(
    profile_name: &str,
    file: &Option<String>,
) -> Result<(), String> {
    println!("Export profile {} to {:?}", profile_name, file);
    Ok(())
}

pub fn import_profile(file: &str) -> Result<(), String> {
    println!("Import profile from {:?}", file);
    Ok(())
}

pub fn install_profile(profile_name: &str) -> Result<(), String> {
    println!("Install profile: {}", profile_name);
    Ok(())
}

pub fn create_profile(profile_name: &str) -> Result<(), String> {
    let path = profile_path(profile_name);

    if path.exists() {
        return Err(format!("Profile '{}' already exists", profile_name));
    }

    let profile = Profile {
        name: profile_name.to_string(),
        packages: Vec::new(),
    };

    let toml_str = toml::to_string(&profile).map_err(|e| e.to_string())?;
    fs::write(path, toml_str).map_err(|e| e.to_string())?;
    println!("Successfully created profile '{}'.", profile_name);

    Ok(())
}

pub fn delete_profile(profile_name: &str) -> Result<(), String> {
    let default_profile = config::get_default_profile();
    if profile_name == default_profile {
        return Err(format!(
            "Cannot delete the default profile '{}'",
            default_profile
        ));
    }

    let path = profile_path(profile_name);

    if !path.exists() {
        return Err(format!("Profile '{}' does not exist", profile_name));
    }

    fs::remove_file(path).map_err(|e| e.to_string())?;
    println!("Successfully deleted profile '{}'.", profile_name);
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
    let default_profile = config::get_default_profile();

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
            if name == default_profile {
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
    use std::{env, fs};

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

        let result = ensure_default_profile("default");
        assert!(result.is_ok());

        let path = profile_path("default");
        assert!(path.exists());

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains(&format!("name = \"{}\"", "default")));
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
        ensure_default_profile("default").unwrap();

        let result = delete_profile("default");
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .contains("Cannot delete the default profile")
        );

        // Ensure it still exists
        assert!(profile_path("default").exists());
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

        ensure_default_profile("default").unwrap();
        create_profile("another").unwrap();

        let mut output = Vec::new();
        list_profiles_to(&mut output).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("Saved profiles:"));
        assert!(output.contains("default"));
        assert!(output.contains("another"));
    }

    #[test]
    #[serial]
    fn test_read_profile_success() {
        let _temp = setup_test_env();
        let profile_name = "test-read";

        create_profile(profile_name).unwrap();

        let result = read_profile(profile_name);
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert_eq!(profile.name, profile_name);
        assert!(profile.packages.is_empty());
    }

    #[test]
    #[serial]
    fn test_read_profile_non_existent() {
        let _temp = setup_test_env();

        let result = read_profile("non-existent");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("does not exist"));
    }

    #[test]
    #[serial]
    fn test_write_profile_success() {
        let _temp = setup_test_env();

        let profile = Profile {
            name: "write-test".to_string(),
            packages: vec![ProfilePackage {
                name: "test-package".to_string(),
                display: Some("Test Package".to_string()),
                installers: HashMap::new(),
                detect: None,
                dependencies: vec![],
            }],
        };

        let result = write_profile(&profile);
        assert!(result.is_ok());

        let path = profile_path(&profile.name);
        assert!(path.exists());

        let read_profile = read_profile(&profile.name).unwrap();
        assert_eq!(read_profile.name, profile.name);
        assert_eq!(read_profile.packages.len(), 1);
        assert_eq!(read_profile.packages[0].name, "test-package");
    }

    #[test]
    #[serial]
    fn test_show_profile_empty() {
        let _temp = setup_test_env();
        let profile_name = "empty-profile";

        create_profile(profile_name).unwrap();

        let result = show_profile(profile_name);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_show_profile_non_existent() {
        let _temp = setup_test_env();

        let result = show_profile("non-existent");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Failed to read profile"));
    }

    #[test]
    #[serial]
    fn test_add_package_to_profile_non_existent_profile() {
        let _temp = setup_test_env();

        let result = add_package_to_profile("non-existent", "some-package");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("does not exist"));
    }
}
