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
