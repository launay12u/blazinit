use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::profile::Profile;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockedPackage {
    pub name: String,
    pub requested_version: Option<String>,
    pub installer: String,
    pub install_command: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LockFile {
    pub profile_name: String,
    pub registry_version: String,
    pub locked_at: String,
    pub packages: Vec<LockedPackage>,
}

pub fn lock_path(profile_name: &str) -> PathBuf {
    let mut path = crate::config::profiles_dir();
    path.push(format!("{}.lock", profile_name));
    path
}

pub fn write_lock(lock: &LockFile) -> Result<(), String> {
    let path = lock_path(&lock.profile_name);
    let toml_str = toml::to_string(lock).map_err(|e| e.to_string())?;
    fs::write(path, toml_str).map_err(|e| e.to_string())
}

pub fn read_lock(profile_name: &str) -> Result<LockFile, String> {
    let path = lock_path(profile_name);
    if !path.exists() {
        return Err(format!(
            "No lock file found for profile '{}'. Run `blazinit install` or `blazinit lock` first.",
            profile_name
        ));
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    toml::from_str(&content).map_err(|e| e.to_string())
}

pub fn read_registry_version() -> Result<String, String> {
    let metadata_path = crate::config::config_dir()
        .join(crate::registry::REGISTRY_DIRNAME)
        .join("metadata.toml");
    if !metadata_path.exists() {
        return Ok("unknown".to_string());
    }
    let content =
        fs::read_to_string(&metadata_path).map_err(|e| e.to_string())?;
    let val: toml::Value =
        toml::from_str(&content).map_err(|e| e.to_string())?;
    Ok(val
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string())
}

pub fn validate_lock_completeness(
    profile: &Profile,
    lock: &LockFile,
) -> Result<(), String> {
    let locked_names: std::collections::HashSet<&str> =
        lock.packages.iter().map(|p| p.name.as_str()).collect();
    let missing: Vec<&str> = profile
        .packages
        .iter()
        .filter(|p| !locked_names.contains(p.name.as_str()))
        .map(|p| p.name.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Lock file is incomplete. Missing packages: {}. Run `blazinit install` or `blazinit lock` to regenerate.",
            missing.join(", ")
        ));
    }
    Ok(())
}

pub fn format_timestamp_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let (year, month, day) = days_to_date(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Howard Hinnant's civil calendar algorithm (days since 1970-01-01)
    let z = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;
    use crate::profile::{PackageRef, Profile};

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        unsafe {
            env::set_var("HOME", temp_dir.path());
            env::set_var("XDG_CONFIG_HOME", temp_dir.path().join(".config"));
        }
        temp_dir
    }

    #[test]
    fn test_format_timestamp_now_format() {
        let ts = format_timestamp_now();
        // Should match YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(ts.len(), 20);
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn test_days_to_date_epoch() {
        // Day 0 = 1970-01-01
        let (y, m, d) = days_to_date(0);
        assert_eq!(y, 1970);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
    }

    #[test]
    fn test_days_to_date_known() {
        // 2026-03-06 = 20518 days since epoch
        let (y, m, d) = days_to_date(20518);
        assert_eq!(y, 2026);
        assert_eq!(m, 3);
        assert_eq!(d, 6);
    }

    #[test]
    #[serial]
    fn test_write_and_read_lock() {
        let _temp = setup_test_env();

        let lock = LockFile {
            profile_name: "default".to_string(),
            registry_version: "2".to_string(),
            locked_at: "2026-03-06T12:00:00Z".to_string(),
            packages: vec![LockedPackage {
                name: "git".to_string(),
                requested_version: Some("2.43.0".to_string()),
                installer: "apt".to_string(),
                install_command: "sudo apt install -y git=2.43.0".to_string(),
            }],
        };

        write_lock(&lock).unwrap();
        let path = lock_path("default");
        assert!(path.exists());

        let read = read_lock("default").unwrap();
        assert_eq!(read.profile_name, "default");
        assert_eq!(read.registry_version, "2");
        assert_eq!(read.packages.len(), 1);
        assert_eq!(read.packages[0].name, "git");
        assert_eq!(
            read.packages[0].requested_version,
            Some("2.43.0".to_string())
        );
        assert_eq!(read.packages[0].installer, "apt");
    }

    #[test]
    #[serial]
    fn test_read_lock_missing() {
        let _temp = setup_test_env();
        let result = read_lock("nonexistent");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("No lock file found"));
    }

    #[test]
    #[serial]
    fn test_validate_lock_completeness_ok() {
        let _temp = setup_test_env();
        let profile = Profile {
            name: "default".to_string(),
            packages: vec![PackageRef {
                name: "git".to_string(),
                installer: None,
                version: None,
            }],
        };
        let lock = LockFile {
            profile_name: "default".to_string(),
            registry_version: "2".to_string(),
            locked_at: "2026-03-06T00:00:00Z".to_string(),
            packages: vec![LockedPackage {
                name: "git".to_string(),
                requested_version: None,
                installer: "apt".to_string(),
                install_command: "sudo apt install -y git".to_string(),
            }],
        };
        assert!(validate_lock_completeness(&profile, &lock).is_ok());
    }

    #[test]
    #[serial]
    fn test_validate_lock_completeness_missing() {
        let _temp = setup_test_env();
        let profile = Profile {
            name: "default".to_string(),
            packages: vec![
                PackageRef {
                    name: "git".to_string(),
                    installer: None,
                    version: None,
                },
                PackageRef {
                    name: "curl".to_string(),
                    installer: None,
                    version: None,
                },
            ],
        };
        let lock = LockFile {
            profile_name: "default".to_string(),
            registry_version: "2".to_string(),
            locked_at: "2026-03-06T00:00:00Z".to_string(),
            packages: vec![LockedPackage {
                name: "git".to_string(),
                requested_version: None,
                installer: "apt".to_string(),
                install_command: "sudo apt install -y git".to_string(),
            }],
        };
        let result = validate_lock_completeness(&profile, &lock);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("curl"));
    }

    #[test]
    #[serial]
    fn test_read_registry_version_missing_file() {
        let _temp = setup_test_env();
        let result = read_registry_version();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "unknown");
    }

    #[test]
    #[serial]
    fn test_read_registry_version_success() {
        let _temp = setup_test_env();
        let reg_dir = crate::config::config_dir().join("registry");
        fs::create_dir_all(&reg_dir).unwrap();
        fs::write(
            reg_dir.join("metadata.toml"),
            "version = \"42\"\npackages = []\n",
        )
        .unwrap();

        let result = read_registry_version();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }
}
