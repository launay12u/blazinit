use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use crate::profile::{Profile, ProfilePackage};

const INSTALLER_PRIORITY: &[&str] =
    &["apt", "dnf", "yum", "pacman", "brew", "winget"];

fn installer_command(installer: &str, pkg_value: &str) -> String {
    match installer {
        "apt" => format!("sudo apt install -y {}", pkg_value),
        "dnf" => format!("sudo dnf install -y {}", pkg_value),
        "yum" => format!("sudo yum install -y {}", pkg_value),
        "pacman" => format!("sudo pacman -S --noconfirm {}", pkg_value),
        "brew" => format!("brew install {}", pkg_value),
        "winget" => format!("winget install {}", pkg_value),
        _ => pkg_value.to_string(),
    }
}

pub fn detect_available_installer() -> Option<String> {
    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    for &installer in INSTALLER_PRIORITY {
        let ok = Command::new(check_cmd)
            .arg(installer)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return Some(installer.to_string());
        }
    }
    None
}

pub fn select_installer(
    pkg: &ProfilePackage,
    installer_flag: &Option<String>,
) -> Result<(String, String), String> {
    // 1. CLI flag
    if let Some(name) = installer_flag {
        if let Some(value) = pkg.installers.get(name.as_str()) {
            return Ok((name.clone(), value.clone()));
        }
        if name == "custom"
            && let Some(cmd) = pkg.installers.get("custom")
        {
            return Ok(("custom".to_string(), cmd.clone()));
        }
        return Err(format!(
            "Installer '{}' not available for package '{}'",
            name, pkg.name
        ));
    }

    // 2. Config preferred installer
    if let Some(name) = crate::config::get_preferred_installer()
        && let Some(value) = pkg.installers.get(name.as_str())
    {
        return Ok((name, value.clone()));
    }

    // 3. Auto-detect first available binary
    if let Some(detected) = detect_available_installer()
        && let Some(value) = pkg.installers.get(detected.as_str())
    {
        return Ok((detected, value.clone()));
    }

    // 4. Fall back to custom
    if let Some(cmd) = pkg.installers.get("custom") {
        return Ok(("custom".to_string(), cmd.clone()));
    }

    Err(format!(
        "No suitable installer found for package '{}'",
        pkg.name
    ))
}

pub fn is_installed(pkg: &ProfilePackage) -> bool {
    let Some(detect_cmd) = &pkg.detect else {
        return false;
    };
    Command::new("sh")
        .arg("-c")
        .arg(detect_cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn topological_sort(
    packages: &[ProfilePackage],
) -> Result<Vec<ProfilePackage>, String> {
    let pkg_map: HashMap<&str, &ProfilePackage> =
        packages.iter().map(|p| (p.name.as_str(), p)).collect();

    let mut visited: HashSet<String> = HashSet::new();
    let mut in_stack: HashSet<String> = HashSet::new();
    let mut order: Vec<String> = Vec::new();

    fn dfs(
        name: &str,
        pkg_map: &HashMap<&str, &ProfilePackage>,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        if in_stack.contains(name) {
            return Err(format!(
                "Circular dependency detected involving '{}'",
                name
            ));
        }
        if visited.contains(name) {
            return Ok(());
        }

        in_stack.insert(name.to_string());

        if let Some(pkg) = pkg_map.get(name) {
            for dep in &pkg.dependencies {
                dfs(dep, pkg_map, visited, in_stack, order)?;
            }
        }

        in_stack.remove(name);
        visited.insert(name.to_string());
        order.push(name.to_string());

        Ok(())
    }

    for pkg in packages {
        dfs(&pkg.name, &pkg_map, &mut visited, &mut in_stack, &mut order)?;
    }

    let sorted = order
        .iter()
        .filter_map(|name| pkg_map.get(name.as_str()).map(|p| (*p).clone()))
        .collect();

    Ok(sorted)
}

pub fn run_install(
    profile: &Profile,
    force: bool,
    installer_flag: &Option<String>,
) -> Result<(), String> {
    if profile.packages.is_empty() {
        println!("No packages to install in profile '{}'.", profile.name);
        return Ok(());
    }

    let ordered = topological_sort(&profile.packages)?;

    let mut installed_count = 0usize;
    let mut skipped_count = 0usize;
    let mut failed_count = 0usize;

    for pkg in &ordered {
        if !force && is_installed(pkg) {
            println!(
                "[skip] {} — already installed",
                pkg.display.as_deref().unwrap_or(&pkg.name)
            );
            skipped_count += 1;
            continue;
        }

        let (installer_name, install_value) =
            match select_installer(pkg, installer_flag) {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!(
                        "[fail] {}: {}",
                        pkg.display.as_deref().unwrap_or(&pkg.name),
                        e
                    );
                    failed_count += 1;
                    continue;
                }
            };

        let cmd_str = if installer_name == "custom" {
            install_value.clone()
        } else {
            installer_command(&installer_name, &install_value)
        };

        println!(
            "[install] {} — running: {}",
            pkg.display.as_deref().unwrap_or(&pkg.name),
            cmd_str
        );

        let status = Command::new("sh").arg("-c").arg(&cmd_str).status();

        match status {
            Ok(s) if s.success() => {
                println!(
                    "[ok] {}",
                    pkg.display.as_deref().unwrap_or(&pkg.name)
                );
                installed_count += 1;
            }
            Ok(s) => {
                eprintln!(
                    "[fail] {} — exited with status {}",
                    pkg.display.as_deref().unwrap_or(&pkg.name),
                    s
                );
                failed_count += 1;
            }
            Err(e) => {
                eprintln!(
                    "[fail] {} — {}",
                    pkg.display.as_deref().unwrap_or(&pkg.name),
                    e
                );
                failed_count += 1;
            }
        }
    }

    println!(
        "\nSummary: {} installed, {} skipped, {} failed",
        installed_count, skipped_count, failed_count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn make_pkg(
        name: &str,
        detect: Option<&str>,
        deps: &[&str],
    ) -> ProfilePackage {
        ProfilePackage {
            name: name.to_string(),
            display: None,
            installers: HashMap::new(),
            detect: detect.map(String::from),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn make_pkg_with_installers(
        name: &str,
        installers: &[(&str, &str)],
    ) -> ProfilePackage {
        ProfilePackage {
            name: name.to_string(),
            display: None,
            installers: installers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            detect: None,
            dependencies: vec![],
        }
    }

    #[test]
    fn test_topological_sort_no_deps() {
        let pkgs =
            vec![make_pkg("git", None, &[]), make_pkg("curl", None, &[])];
        let sorted = topological_sort(&pkgs).unwrap();
        assert_eq!(sorted.len(), 2);
    }

    #[test]
    fn test_topological_sort_with_deps() {
        let pkgs =
            vec![make_pkg("curl", None, &["git"]), make_pkg("git", None, &[])];
        let sorted = topological_sort(&pkgs).unwrap();
        let names: Vec<_> = sorted.iter().map(|p| p.name.as_str()).collect();
        let git_pos = names.iter().position(|&n| n == "git").unwrap();
        let curl_pos = names.iter().position(|&n| n == "curl").unwrap();
        assert!(git_pos < curl_pos);
    }

    #[test]
    fn test_topological_sort_cycle_detected() {
        let pkgs =
            vec![make_pkg("a", None, &["b"]), make_pkg("b", None, &["a"])];
        let result = topological_sort(&pkgs);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Circular dependency"));
    }

    #[test]
    fn test_select_installer_flag_override() {
        let pkg =
            make_pkg_with_installers("git", &[("apt", "git"), ("brew", "git")]);
        let result = select_installer(&pkg, &Some("apt".to_string()));
        assert!(result.is_ok());
        let (name, _) = result.unwrap();
        assert_eq!(name, "apt");
    }

    #[test]
    fn test_select_installer_flag_not_available() {
        let pkg = make_pkg_with_installers("git", &[("apt", "git")]);
        let result = select_installer(&pkg, &Some("brew".to_string()));
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("not available"));
    }

    #[test]
    fn test_select_installer_custom_fallback() {
        let pkg = make_pkg_with_installers(
            "mypkg",
            &[("custom", "install-mypkg.sh")],
        );
        let result = select_installer(&pkg, &None);
        assert!(result.is_ok());
        let (name, _) = result.unwrap();
        assert_eq!(name, "custom");
    }

    #[test]
    fn test_select_installer_no_installer_available() {
        let pkg = make_pkg("mypkg", None, &[]);
        let result = select_installer(&pkg, &None);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("No suitable installer"));
    }

    #[test]
    fn test_is_installed_no_detect() {
        let pkg = make_pkg("mypkg", None, &[]);
        assert!(!is_installed(&pkg));
    }

    #[test]
    fn test_is_installed_true_command() {
        let pkg = make_pkg("mypkg", Some("true"), &[]);
        assert!(is_installed(&pkg));
    }

    #[test]
    fn test_is_installed_false_command() {
        let pkg = make_pkg("mypkg", Some("false"), &[]);
        assert!(!is_installed(&pkg));
    }
}
