use std::{fs, path::PathBuf};

use dirs_next;
use include_dir::{Dir, include_dir};

use crate::{
    profile::{PROFILE_DIRNAME, ensure_default_profile},
    registry::update_registry_version_if_needed,
};

pub static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

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
    fs::create_dir_all(&base)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    ensure_default_profile()?;
    update_registry_version_if_needed()?;

    Ok(())
}
