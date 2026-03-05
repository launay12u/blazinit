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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageRef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installer: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Profile {
    pub name: String,
    pub packages: Vec<PackageRef>,
}

pub const PROFILE_DIRNAME: &str = "profiles";

pub fn ensure_default_profile(profile_name: &str) -> Result<(), String> {
    let path = profile_path(profile_name);
    if !path.exists() {
        log::debug!(
            "default profile '{}' not found, creating it",
            profile_name
        );
        let profile = Profile {
            name: profile_name.to_string(),
            packages: Vec::new(),
        };
        let toml_str = toml::to_string(&profile).map_err(|e| e.to_string())?;
        fs::write(path, toml_str).map_err(|e| e.to_string())?;
        log::info!("created default profile '{}'", profile_name);
    } else {
        log::debug!("default profile '{}' already exists", profile_name);
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
    log::debug!("reading profile '{}' from {:?}", profile_name, path);
    if !path.exists() {
        log::error!("profile '{}' does not exist at {:?}", profile_name, path);
        return Err(format!("Profile '{}' does not exist.", profile_name));
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let profile: Profile =
        toml::from_str(&content).map_err(|e| e.to_string())?;
    log::debug!(
        "profile '{}' loaded ({} packages)",
        profile_name,
        profile.packages.len()
    );
    Ok(profile)
}

pub fn write_profile(profile: &Profile) -> Result<(), String> {
    let path = profile_path(&profile.name);
    log::debug!(
        "writing profile '{}' ({} packages) to {:?}",
        profile.name,
        profile.packages.len(),
        path
    );
    let toml_str = toml::to_string(profile).map_err(|e| e.to_string())?;
    fs::write(path, toml_str).map_err(|e| e.to_string())
}

pub fn add_package_to_profile(
    profile_name: &str,
    package_name: &str,
    installer: Option<String>,
) -> Result<(), String> {
    log::debug!(
        "adding package '{}' to profile '{}' (installer={:?})",
        package_name,
        profile_name,
        installer
    );
    let mut profile = read_profile(profile_name)?;

    if profile.packages.iter().any(|p| p.name == package_name) {
        log::warn!(
            "package '{}' already present in profile '{}'",
            package_name,
            profile_name
        );
        return Err(format!(
            "Package '{}' is already present in profile '{}'",
            package_name, profile_name
        ));
    }

    log::debug!("checking package '{}' in registry", package_name);
    if !crate::registry::is_package_in_registry(package_name)? {
        log::error!("package '{}' not found in registry", package_name);
        return Err(format!(
            "Package '{}' not found in registry",
            package_name
        ));
    }

    profile.packages.push(PackageRef {
        name: package_name.to_string(),
        installer,
    });
    profile.packages.sort_by(|a, b| a.name.cmp(&b.name));
    write_profile(&profile)?;

    log::info!(
        "added package '{}' to profile '{}'",
        package_name,
        profile_name
    );
    println!("Adding to profile {}:", profile_name.cyan().bold());
    println!("  {} {}", "+".green().bold(), package_name.cyan());
    println!("{}", "Successfully added 1 package.".green());

    Ok(())
}

pub fn show_profile(profile_name: &str) -> Result<(), String> {
    let p = read_profile(profile_name)
        .map_err(|e| format!("Failed to read profile: {}", e))?;
    println!("{} {}", "Profile:".bold(), p.name.cyan().bold());
    if p.packages.is_empty() {
        println!("  {}", "No packages in this profile.".dimmed());
    } else {
        println!("  {}", "Packages:".bold());
        for pkg_ref in &p.packages {
            let display = crate::registry::get_package_details(&pkg_ref.name)
                .ok()
                .and_then(|d| d.display)
                .unwrap_or_else(|| pkg_ref.name.clone());
            if let Some(installer) = &pkg_ref.installer {
                println!(
                    "  - {} {}",
                    display.cyan(),
                    format!("(installer: {})", installer).dimmed()
                );
            } else {
                println!("  - {}", display.cyan());
            }
        }
    }
    Ok(())
}

pub fn remove_package_from_profile(
    profile_name: &str,
    package_name: &str,
) -> Result<(), String> {
    log::debug!(
        "removing package '{}' from profile '{}'",
        package_name,
        profile_name
    );
    let mut profile = read_profile(profile_name)?;

    let initial_len = profile.packages.len();
    profile.packages.retain(|pkg| pkg.name != package_name);

    if profile.packages.len() == initial_len {
        log::warn!(
            "package '{}' not found in profile '{}'",
            package_name,
            profile_name
        );
        return Err(format!(
            "Package '{}' is not present in profile '{}'",
            package_name, profile_name
        ));
    }

    write_profile(&profile)?;
    log::info!(
        "removed package '{}' from profile '{}'",
        package_name,
        profile_name
    );
    println!(
        "{} '{}' from profile '{}'",
        "Successfully removed".green(),
        package_name.cyan(),
        profile_name.cyan()
    );

    Ok(())
}

pub fn export_profile(
    profile_name: &str,
    file: &Option<String>,
) -> Result<(), String> {
    let src = profile_path(profile_name);
    log::debug!("exporting profile '{}' from {:?}", profile_name, src);
    if !src.exists() {
        log::error!("export failed: profile '{}' does not exist", profile_name);
        return Err(format!("Profile '{}' does not exist", profile_name));
    }

    match file {
        Some(dest) => {
            fs::copy(&src, dest)
                .map_err(|e| format!("Failed to export profile: {}", e))?;
            log::info!("exported profile '{}' to '{}'", profile_name, dest);
            println!(
                "{} '{}' exported to '{}'",
                "Profile".green(),
                profile_name.cyan(),
                dest.cyan()
            );
        }
        None => {
            log::debug!("exporting profile '{}' to stdout", profile_name);
            let content = fs::read_to_string(&src)
                .map_err(|e| format!("Failed to read profile: {}", e))?;
            print!("{}", content);
        }
    }

    Ok(())
}

pub fn import_profile(file: &str) -> Result<(), String> {
    log::debug!("importing profile from '{}'", file);
    let content = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file '{}': {}", file, e))?;

    let profile: Profile = toml::from_str(&content)
        .map_err(|e| format!("Invalid profile file: {}", e))?;

    let dest = profile_path(&profile.name);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&dest)
        .and_then(|mut f| std::io::Write::write_all(&mut f, content.as_bytes()))
        .map_err(|_| {
            log::error!("import failed: profile '{}' already exists", profile.name);
            format!(
                "Profile '{}' already exists. Delete it first or rename the import file.",
                profile.name
            )
        })?;

    log::info!("imported profile '{}' from '{}'", profile.name, file);
    println!(
        "{} '{}'.",
        "Profile imported successfully:".green(),
        profile.name.cyan()
    );
    Ok(())
}

pub fn install_profile(
    profile_name: &str,
    force: bool,
    installer: &Option<String>,
    dry_run: bool,
) -> Result<(), String> {
    log::info!(
        "installing profile '{}' (force={}, dry_run={}, installer={:?})",
        profile_name,
        force,
        dry_run,
        installer
    );
    let profile = read_profile(profile_name)?;
    crate::installer::run_install(&profile, force, installer, dry_run)
}

pub fn create_profile(profile_name: &str) -> Result<(), String> {
    let path = profile_path(profile_name);
    log::debug!("creating profile '{}' at {:?}", profile_name, path);

    if path.exists() {
        log::warn!("create failed: profile '{}' already exists", profile_name);
        return Err(format!("Profile '{}' already exists", profile_name));
    }

    let profile = Profile {
        name: profile_name.to_string(),
        packages: Vec::new(),
    };

    let toml_str = toml::to_string(&profile).map_err(|e| e.to_string())?;
    fs::write(path, toml_str).map_err(|e| e.to_string())?;
    log::info!("profile '{}' created", profile_name);
    println!(
        "{} '{}'.",
        "Successfully created profile".green(),
        profile_name.cyan()
    );

    Ok(())
}

pub fn delete_profile(profile_name: &str) -> Result<(), String> {
    let default_profile = config::get_default_profile();
    if profile_name == default_profile {
        log::error!("delete failed: '{}' is the default profile", profile_name);
        return Err(format!(
            "Cannot delete the default profile '{}'",
            default_profile
        ));
    }

    let path = profile_path(profile_name);
    log::debug!("deleting profile '{}' at {:?}", profile_name, path);

    if !path.exists() {
        log::error!("delete failed: profile '{}' does not exist", profile_name);
        return Err(format!("Profile '{}' does not exist", profile_name));
    }

    fs::remove_file(path).map_err(|e| e.to_string())?;
    log::info!("profile '{}' deleted", profile_name);
    println!(
        "{} '{}'.",
        "Successfully deleted profile".green(),
        profile_name.cyan()
    );
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
            packages: vec![PackageRef {
                name: "test-package".to_string(),
                installer: None,
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
    fn test_add_package_not_in_registry() {
        let _temp = setup_test_env();
        let profile_name = "test-registry-validation";

        let reg_dir = crate::config::config_dir().join("registry");
        std::fs::create_dir_all(&reg_dir).unwrap();
        std::fs::write(
            reg_dir.join("metadata.toml"),
            "version = \"2\"\npackages = []\n",
        )
        .unwrap();
        std::fs::write(
            reg_dir.join("existing.toml"),
            "display = \"Existing\"\n",
        )
        .unwrap();

        create_profile(profile_name).unwrap();

        let result =
            add_package_to_profile(profile_name, "nonexistent-pkg", None);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("not found in registry"));
    }

    #[test]
    #[serial]
    fn test_add_package_to_profile_non_existent_profile() {
        let _temp = setup_test_env();

        let result =
            add_package_to_profile("non-existent", "some-package", None);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("does not exist"));
    }

    #[test]
    #[serial]
    fn test_remove_package_success() {
        let _temp = setup_test_env();
        let profile_name = "test-remove";

        let profile = Profile {
            name: profile_name.to_string(),
            packages: vec![
                PackageRef {
                    name: "package1".to_string(),
                    installer: None,
                },
                PackageRef {
                    name: "package2".to_string(),
                    installer: None,
                },
            ],
        };
        write_profile(&profile).unwrap();

        // Remove one package
        let result = remove_package_from_profile(profile_name, "package1");
        assert!(result.is_ok());

        // Verify package was removed
        let updated_profile = read_profile(profile_name).unwrap();
        assert_eq!(updated_profile.packages.len(), 1);
        assert_eq!(updated_profile.packages[0].name, "package2");
    }

    #[test]
    #[serial]
    fn test_remove_package_not_present() {
        let _temp = setup_test_env();
        let profile_name = "test-remove-not-present";

        // Create empty profile
        create_profile(profile_name).unwrap();

        // Try to remove non-existent package
        let result = remove_package_from_profile(profile_name, "non-existent");
        assert!(result.is_err());
        let error_msg = result.err().unwrap();
        assert!(error_msg.contains("is not present in profile"));
        assert!(error_msg.contains("non-existent"));
        assert!(error_msg.contains(profile_name));
    }

    #[test]
    #[serial]
    fn test_remove_package_from_non_existent_profile() {
        let _temp = setup_test_env();

        let result =
            remove_package_from_profile("non-existent", "some-package");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("does not exist"));
    }

    #[test]
    #[serial]
    fn test_remove_package_multiple_times() {
        let _temp = setup_test_env();
        let profile_name = "test-remove-multiple";

        let profile = Profile {
            name: profile_name.to_string(),
            packages: vec![PackageRef {
                name: "brew".to_string(),
                installer: None,
            }],
        };
        write_profile(&profile).unwrap();

        // First removal should succeed
        let result = remove_package_from_profile(profile_name, "brew");
        assert!(result.is_ok());

        // Second removal should fail
        let result = remove_package_from_profile(profile_name, "brew");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("is not present in profile"));
    }

    #[test]
    #[serial]
    fn test_add_package_already_present() {
        let _temp = setup_test_env();
        let profile_name = "test-add-duplicate";

        let profile = Profile {
            name: profile_name.to_string(),
            packages: vec![PackageRef {
                name: "git".to_string(),
                installer: None,
            }],
        };
        write_profile(&profile).unwrap();

        // Try to add the same package again
        let result = add_package_to_profile(profile_name, "git", None);
        assert!(result.is_err());
        let error_msg = result.err().unwrap();
        assert!(error_msg.contains("is already present in profile"));
        assert!(error_msg.contains("git"));
        assert!(error_msg.contains(profile_name));
    }

    #[test]
    #[serial]
    fn test_export_to_file() {
        let _temp = setup_test_env();
        let profile_name = "test-export";
        create_profile(profile_name).unwrap();

        let dest = _temp.path().join("exported.toml");
        let dest_str = dest.to_str().unwrap().to_string();
        let result = export_profile(profile_name, &Some(dest_str));
        assert!(result.is_ok());
        assert!(dest.exists());

        let content = fs::read_to_string(&dest).unwrap();
        assert!(content.contains(profile_name));
    }

    #[test]
    #[serial]
    fn test_export_non_existent_profile() {
        let _temp = setup_test_env();
        let result = export_profile("ghost", &None);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("does not exist"));
    }

    #[test]
    #[serial]
    fn test_import_roundtrip() {
        let _temp = setup_test_env();
        let profile_name = "roundtrip-profile";
        create_profile(profile_name).unwrap();

        let dest = _temp.path().join("roundtrip.toml");
        let dest_str = dest.to_str().unwrap().to_string();
        export_profile(profile_name, &Some(dest_str.clone())).unwrap();

        delete_profile(profile_name).unwrap_or_default();
        // Use a different profile name so delete doesn't block (default
        // protection) The exported file still has original name so just
        // delete the file directly
        if profile_path(profile_name).exists() {
            fs::remove_file(profile_path(profile_name)).unwrap();
        }

        let result = import_profile(&dest_str);
        assert!(result.is_ok());
        assert!(profile_path(profile_name).exists());
    }

    #[test]
    #[serial]
    fn test_import_duplicate_fails() {
        let _temp = setup_test_env();
        let profile_name = "dup-import";
        create_profile(profile_name).unwrap();

        let dest = _temp.path().join("dup.toml");
        let dest_str = dest.to_str().unwrap().to_string();
        export_profile(profile_name, &Some(dest_str.clone())).unwrap();

        let result = import_profile(&dest_str);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("already exists"));
    }

    #[test]
    #[serial]
    fn test_import_invalid_toml_fails() {
        let _temp = setup_test_env();
        let bad_file = _temp.path().join("bad.toml");
        fs::write(&bad_file, "not valid toml ][[[").unwrap();
        let result = import_profile(bad_file.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Invalid profile file"));
    }

    #[test]
    #[serial]
    fn test_add_package_multiple_times() {
        let _temp = setup_test_env();
        let profile_name = "test-add-multiple";

        let profile = Profile {
            name: profile_name.to_string(),
            packages: vec![PackageRef {
                name: "docker".to_string(),
                installer: None,
            }],
        };
        write_profile(&profile).unwrap();

        // First attempt should fail
        let result = add_package_to_profile(profile_name, "docker", None);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .contains("is already present in profile")
        );

        // Verify the profile still has only one package
        let updated_profile = read_profile(profile_name).unwrap();
        assert_eq!(updated_profile.packages.len(), 1);
        assert_eq!(updated_profile.packages[0].name, "docker");
    }
}
