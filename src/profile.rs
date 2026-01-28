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
    let path = profiles_dir();

    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(_) => {
            println!("No profiles found.");
            return;
        }
    };

    println!("Saved profiles:");

    for entry in entries {
        if let Ok(entry) = entry
            && let Some(name) =
                entry.path().file_stem().and_then(|s| s.to_str())
        {
            if name == DEFAULT_PROFILE {
                println!("{} (default)", name.green());
            } else {
                println!("{}", name);
            }
        }
    }
}
