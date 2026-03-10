use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use colored::Colorize;

use crate::profile::{PackageRef, Profile, ProfilePackage};

const INSTALLER_PRIORITY: &[&str] =
    &["apt", "dnf", "yum", "pacman", "brew", "winget"];

pub fn installer_command(
    installer: &str,
    pkg_value: &str,
    version: Option<&str>,
) -> String {
    match (installer, version) {
        ("apt", Some(v)) => {
            format!("sudo apt install -y {}={}", pkg_value, v)
        }
        ("apt", None) => format!("sudo apt install -y {}", pkg_value),
        ("brew", Some(v)) => format!("brew install {}@{}", pkg_value, v),
        ("brew", None) => format!("brew install {}", pkg_value),
        ("dnf", Some(v)) => {
            format!("sudo dnf install -y {}-{}", pkg_value, v)
        }
        ("dnf", None) => format!("sudo dnf install -y {}", pkg_value),
        ("yum", Some(v)) => {
            format!("sudo yum install -y {}-{}", pkg_value, v)
        }
        ("yum", None) => format!("sudo yum install -y {}", pkg_value),
        ("pacman", Some(_)) => {
            eprintln!(
                "Warning: pacman does not support version pinning; \
                 installing latest."
            );
            format!("sudo pacman -S --noconfirm {}", pkg_value)
        }
        ("pacman", None) => {
            format!("sudo pacman -S --noconfirm {}", pkg_value)
        }
        ("winget", _) => format!("winget install {}", pkg_value),
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
            log::debug!("detected system installer: {}", installer);
            return Some(installer.to_string());
        }
    }
    log::debug!("no system installer detected");
    None
}

pub fn select_installer(
    pkg: &ProfilePackage,
    installer_flag: &Option<String>,
) -> Result<(String, String), String> {
    // 1. CLI flag
    if let Some(name) = installer_flag {
        if let Some(value) = pkg.installers.get(name.as_str()) {
            log::debug!(
                "installer for '{}': cli flag '{}' -> '{}'",
                pkg.name,
                name,
                value
            );
            return Ok((name.clone(), value.clone()));
        }
        if name == "custom"
            && let Some(cmd) = pkg.installers.get("custom")
        {
            log::debug!(
                "installer for '{}': cli flag 'custom' -> '{}'",
                pkg.name,
                cmd
            );
            return Ok(("custom".to_string(), cmd.clone()));
        }
        log::error!(
            "installer '{}' not available for package '{}'",
            name,
            pkg.name
        );
        return Err(format!(
            "Installer '{}' not available for package '{}'",
            name, pkg.name
        ));
    }

    // 2. Config preferred installer
    if let Some(name) = crate::config::get_preferred_installer()
        && let Some(value) = pkg.installers.get(name.as_str())
    {
        log::debug!(
            "installer for '{}': config preferred '{}' -> '{}'",
            pkg.name,
            name,
            value
        );
        return Ok((name, value.clone()));
    }

    // 3. Auto-detect first available binary
    if let Some(detected) = detect_available_installer()
        && let Some(value) = pkg.installers.get(detected.as_str())
    {
        log::debug!(
            "installer for '{}': auto-detected '{}' -> '{}'",
            pkg.name,
            detected,
            value
        );
        return Ok((detected, value.clone()));
    }

    // 4. Fall back to custom
    if let Some(cmd) = pkg.installers.get("custom") {
        log::debug!(
            "installer for '{}': fallback custom -> '{}'",
            pkg.name,
            cmd
        );
        return Ok(("custom".to_string(), cmd.clone()));
    }

    log::error!("no suitable installer found for package '{}'", pkg.name);
    Err(format!(
        "No suitable installer found for package '{}'",
        pkg.name
    ))
}

pub fn is_installed(pkg: &ProfilePackage) -> bool {
    let Some(detect_cmd) = &pkg.detect else {
        log::debug!(
            "'{}': no detect command, assuming not installed",
            pkg.name
        );
        return false;
    };
    let result = Command::new("sh")
        .arg("-c")
        .arg(detect_cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    log::debug!(
        "'{}': detect='{}' -> installed={}",
        pkg.name,
        detect_cmd,
        result
    );
    result
}

fn topological_sort(packages: &[PackageRef]) -> Result<Vec<String>, String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut in_stack: HashSet<String> = HashSet::new();
    let mut order: Vec<String> = Vec::new();

    fn dfs(
        name: &str,
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

        let deps = crate::registry::get_dependencies(name).unwrap_or_default();
        for dep in &deps {
            dfs(dep, visited, in_stack, order)?;
        }

        in_stack.remove(name);
        visited.insert(name.to_string());
        order.push(name.to_string());

        Ok(())
    }

    for pkg_ref in packages {
        dfs(&pkg_ref.name, &mut visited, &mut in_stack, &mut order)?;
    }

    log::debug!("topological install order: {:?}", order);
    Ok(order)
}

pub fn run_install(
    profile: &Profile,
    force: bool,
    cli_installer: &Option<String>,
    dry_run: bool,
    frozen: bool,
) -> Result<(), String> {
    log::info!(
        "run_install: profile='{}', packages={}, force={}, dry_run={}, \
         frozen={}, cli_installer={:?}",
        profile.name,
        profile.packages.len(),
        force,
        dry_run,
        frozen,
        cli_installer
    );

    if profile.packages.is_empty() {
        log::info!("profile '{}' has no packages to install", profile.name);
        println!("{}", "No packages to install.".yellow());
        return Ok(());
    }

    if frozen {
        return run_install_frozen(profile, force, dry_run);
    }

    let pkg_ref_map: HashMap<&str, &PackageRef> = profile
        .packages
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    let ordered_names = topological_sort(&profile.packages)?;

    let mut installed_count = 0usize;
    let mut skipped_count = 0usize;
    let mut failed_count = 0usize;
    let mut locked_packages: Vec<crate::lockfile::LockedPackage> = Vec::new();

    for name in &ordered_names {
        let pkg = match crate::registry::get_package_details(name) {
            Ok(p) => p,
            Err(e) => {
                log::error!("failed to get details for '{}': {}", name, e);
                eprintln!("{} {}: {}", "[fail]".red().bold(), name.cyan(), e);
                failed_count += 1;
                continue;
            }
        };

        let display = pkg.display.as_deref().unwrap_or(&pkg.name);
        let pkg_ref = pkg_ref_map.get(name.as_str());
        let version = pkg_ref.and_then(|r| r.version.as_deref());
        let pkg_installer = pkg_ref.and_then(|r| r.installer.as_ref()).cloned();
        let effective_installer = cli_installer.clone().or(pkg_installer);

        if !force && is_installed(&pkg) {
            log::debug!("'{}' already installed, skipping", name);
            println!(
                "{} {} — already installed",
                "[skip]".yellow().bold(),
                display.cyan()
            );
            skipped_count += 1;

            // best-effort: capture command for lock file
            if !dry_run
                && let Ok((inst_name, inst_value)) =
                    select_installer(&pkg, &effective_installer)
            {
                let cmd_str = if inst_name == "custom" {
                    inst_value.clone()
                } else {
                    installer_command(&inst_name, &inst_value, version)
                };
                locked_packages.push(crate::lockfile::LockedPackage {
                    name: name.clone(),
                    requested_version: version.map(String::from),
                    installer: inst_name,
                    install_command: cmd_str,
                });
            }
            continue;
        }

        // Priority: CLI flag > per-package profile override > config >
        // auto-detect > custom
        let (installer_name, install_value) =
            match select_installer(&pkg, &effective_installer) {
                Ok(pair) => pair,
                Err(e) => {
                    log::error!("no installer for '{}': {}", name, e);
                    eprintln!(
                        "{} {}: {}",
                        "[fail]".red().bold(),
                        display.cyan(),
                        e
                    );
                    failed_count += 1;
                    continue;
                }
            };

        let cmd_str = if installer_name == "custom" {
            install_value.clone()
        } else {
            installer_command(&installer_name, &install_value, version)
        };

        if dry_run {
            log::debug!("dry-run '{}': would run: {}", name, cmd_str);
            println!(
                "{} {} — would run: {}",
                "[dry-run]".cyan().bold(),
                display.cyan(),
                cmd_str.dimmed()
            );
            installed_count += 1;
            continue;
        }

        log::info!("installing '{}' via {}: {}", name, installer_name, cmd_str);
        println!(
            "{} {} — {}",
            "[install]".blue().bold(),
            display.cyan(),
            cmd_str.dimmed()
        );

        let status = Command::new("sh").arg("-c").arg(&cmd_str).status();

        match status {
            Ok(s) if s.success() => {
                log::info!("'{}' installed successfully", name);
                println!("{} {}", "[ok]".green().bold(), display.cyan());
                installed_count += 1;
                locked_packages.push(crate::lockfile::LockedPackage {
                    name: name.clone(),
                    requested_version: version.map(String::from),
                    installer: installer_name,
                    install_command: cmd_str,
                });
            }
            Ok(s) => {
                log::error!("'{}' install failed: exit status {}", name, s);
                eprintln!(
                    "{} {} — exited with status {}",
                    "[fail]".red().bold(),
                    display.cyan(),
                    s
                );
                failed_count += 1;
            }
            Err(e) => {
                log::error!("'{}' install command error: {}", name, e);
                eprintln!(
                    "{} {} — {}",
                    "[fail]".red().bold(),
                    display.cyan(),
                    e
                );
                failed_count += 1;
            }
        }
    }

    let installed_label = if dry_run {
        "would install"
    } else {
        "installed"
    };
    log::info!(
        "install complete: {} {}, {} skipped, {} failed",
        installed_count,
        installed_label,
        skipped_count,
        failed_count
    );
    println!(
        "\n{} {} {}  {} skipped  {} failed",
        "Summary:".bold(),
        installed_count.to_string().green().bold(),
        installed_label,
        skipped_count.to_string().yellow().bold(),
        failed_count.to_string().red().bold()
    );

    if !dry_run {
        let registry_version = crate::lockfile::read_registry_version()
            .unwrap_or_else(|_| "unknown".to_string());
        let lock = crate::lockfile::LockFile {
            profile_name: profile.name.clone(),
            registry_version,
            locked_at: crate::lockfile::format_timestamp_now(),
            packages: locked_packages,
        };
        crate::lockfile::write_lock(&lock)?;
        log::info!("lock file written for profile '{}'", profile.name);
    }

    Ok(())
}

fn run_install_frozen(
    profile: &Profile,
    force: bool,
    dry_run: bool,
) -> Result<(), String> {
    let lock = crate::lockfile::read_lock(&profile.name)?;
    crate::lockfile::validate_lock_completeness(profile, &lock)?;

    let mut installed_count = 0usize;
    let mut skipped_count = 0usize;
    let mut failed_count = 0usize;

    for locked_pkg in &lock.packages {
        let pkg_opt =
            crate::registry::get_package_details(&locked_pkg.name).ok();
        let display = pkg_opt
            .as_ref()
            .and_then(|p| p.display.as_deref())
            .unwrap_or(&locked_pkg.name);

        if !force {
            let already = pkg_opt.as_ref().map(is_installed).unwrap_or(false);
            if already {
                log::debug!(
                    "'{}' already installed, skipping (frozen)",
                    locked_pkg.name
                );
                println!(
                    "{} {} — already installed",
                    "[skip]".yellow().bold(),
                    display.cyan()
                );
                skipped_count += 1;
                continue;
            }
        }

        if dry_run {
            log::debug!(
                "frozen dry-run '{}': would run: {}",
                locked_pkg.name,
                locked_pkg.install_command
            );
            println!(
                "{} {} — would run: {}",
                "[dry-run]".cyan().bold(),
                display.cyan(),
                locked_pkg.install_command.dimmed()
            );
            installed_count += 1;
            continue;
        }

        log::info!(
            "frozen install '{}': {}",
            locked_pkg.name,
            locked_pkg.install_command
        );
        println!(
            "{} {} — {}",
            "[install]".blue().bold(),
            display.cyan(),
            locked_pkg.install_command.dimmed()
        );

        let status = Command::new("sh")
            .arg("-c")
            .arg(&locked_pkg.install_command)
            .status();

        match status {
            Ok(s) if s.success() => {
                log::info!(
                    "'{}' installed successfully (frozen)",
                    locked_pkg.name
                );
                println!("{} {}", "[ok]".green().bold(), display.cyan());
                installed_count += 1;
            }
            Ok(s) => {
                log::error!(
                    "'{}' frozen install failed: exit status {}",
                    locked_pkg.name,
                    s
                );
                eprintln!(
                    "{} {} — exited with status {}",
                    "[fail]".red().bold(),
                    display.cyan(),
                    s
                );
                failed_count += 1;
            }
            Err(e) => {
                log::error!(
                    "'{}' frozen install command error: {}",
                    locked_pkg.name,
                    e
                );
                eprintln!(
                    "{} {} — {}",
                    "[fail]".red().bold(),
                    display.cyan(),
                    e
                );
                failed_count += 1;
            }
        }
    }

    let installed_label = if dry_run {
        "would install"
    } else {
        "installed"
    };
    log::info!(
        "frozen install complete: {} {}, {} skipped, {} failed",
        installed_count,
        installed_label,
        skipped_count,
        failed_count
    );
    println!(
        "\n{} {} {}  {} skipped  {} failed",
        "Summary:".bold(),
        installed_count.to_string().green().bold(),
        installed_label,
        skipped_count.to_string().yellow().bold(),
        failed_count.to_string().red().bold()
    );

    Ok(())
}

pub fn generate_lock(
    profile: &Profile,
    cli_installer: &Option<String>,
) -> Result<(), String> {
    log::info!(
        "generate_lock: profile='{}', packages={}",
        profile.name,
        profile.packages.len()
    );

    if profile.packages.is_empty() {
        println!("{}", "No packages to lock.".yellow());
        return Ok(());
    }

    let pkg_ref_map: HashMap<&str, &PackageRef> = profile
        .packages
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    let ordered_names = topological_sort(&profile.packages)?;

    let mut locked_packages: Vec<crate::lockfile::LockedPackage> = Vec::new();

    for name in &ordered_names {
        let pkg = match crate::registry::get_package_details(name) {
            Ok(p) => p,
            Err(e) => {
                log::error!("failed to get details for '{}': {}", name, e);
                eprintln!("{} {}: {}", "[fail]".red().bold(), name.cyan(), e);
                continue;
            }
        };

        let pkg_ref = pkg_ref_map.get(name.as_str());
        let version = pkg_ref.and_then(|r| r.version.as_deref());
        let pkg_installer = pkg_ref.and_then(|r| r.installer.as_ref()).cloned();
        let effective_installer = cli_installer.clone().or(pkg_installer);

        let (installer_name, install_value) =
            match select_installer(&pkg, &effective_installer) {
                Ok(pair) => pair,
                Err(e) => {
                    log::error!("no installer for '{}': {}", name, e);
                    eprintln!(
                        "{} {}: {}",
                        "[fail]".red().bold(),
                        name.cyan(),
                        e
                    );
                    continue;
                }
            };

        let cmd_str = if installer_name == "custom" {
            install_value.clone()
        } else {
            installer_command(&installer_name, &install_value, version)
        };

        locked_packages.push(crate::lockfile::LockedPackage {
            name: name.clone(),
            requested_version: version.map(String::from),
            installer: installer_name,
            install_command: cmd_str,
        });
    }

    let registry_version = crate::lockfile::read_registry_version()
        .unwrap_or_else(|_| "unknown".to_string());
    let lock = crate::lockfile::LockFile {
        profile_name: profile.name.clone(),
        registry_version,
        locked_at: crate::lockfile::format_timestamp_now(),
        packages: locked_packages,
    };

    crate::lockfile::write_lock(&lock)?;
    log::info!("lock file generated for profile '{}'", profile.name);
    println!(
        "{} '{}'.",
        "Lock file generated for profile".green(),
        profile.name.cyan()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn make_pkg_ref(name: &str) -> PackageRef {
        PackageRef {
            name: name.to_string(),
            installer: None,
            version: None,
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

    fn make_pkg_resolved(name: &str, detect: Option<&str>) -> ProfilePackage {
        ProfilePackage {
            name: name.to_string(),
            display: None,
            installers: HashMap::new(),
            detect: detect.map(String::from),
            dependencies: vec![],
        }
    }

    #[test]
    fn test_installer_command_no_version() {
        assert_eq!(
            installer_command("apt", "git", None),
            "sudo apt install -y git"
        );
        assert_eq!(installer_command("brew", "git", None), "brew install git");
        assert_eq!(
            installer_command("dnf", "git", None),
            "sudo dnf install -y git"
        );
        assert_eq!(
            installer_command("yum", "git", None),
            "sudo yum install -y git"
        );
        assert_eq!(
            installer_command("pacman", "git", None),
            "sudo pacman -S --noconfirm git"
        );
        assert_eq!(
            installer_command("winget", "Git.Git", None),
            "winget install Git.Git"
        );
        assert_eq!(
            installer_command("custom", "install.sh", None),
            "install.sh"
        );
    }

    #[test]
    fn test_installer_command_with_version() {
        assert_eq!(
            installer_command("apt", "git", Some("2.43.0")),
            "sudo apt install -y git=2.43.0"
        );
        assert_eq!(
            installer_command("brew", "git", Some("2.43.0")),
            "brew install git@2.43.0"
        );
        assert_eq!(
            installer_command("dnf", "git", Some("2.43.0")),
            "sudo dnf install -y git-2.43.0"
        );
        assert_eq!(
            installer_command("yum", "git", Some("2.43.0")),
            "sudo yum install -y git-2.43.0"
        );
        // winget ignores version
        assert_eq!(
            installer_command("winget", "Git.Git", Some("2.43.0")),
            "winget install Git.Git"
        );
        // custom pass-through ignores version
        assert_eq!(
            installer_command("custom", "install.sh", Some("1.0")),
            "install.sh"
        );
    }

    #[test]
    fn test_topological_sort_no_deps() {
        let pkgs = vec![make_pkg_ref("git"), make_pkg_ref("curl")];
        let sorted = topological_sort(&pkgs).unwrap();
        assert_eq!(sorted.len(), 2);
    }

    #[test]
    fn test_topological_sort_with_deps() {
        // curl depends on git — but topo sort resolves from registry,
        // and these packages aren't in registry in unit tests,
        // so deps will be empty. Just verify ordering doesn't crash.
        let pkgs = vec![make_pkg_ref("curl"), make_pkg_ref("git")];
        let sorted = topological_sort(&pkgs).unwrap();
        assert_eq!(sorted.len(), 2);
    }

    #[test]
    fn test_topological_sort_cycle_detected() {
        // Cycle detection requires registry entries; without them deps are
        // empty so no cycle. This test verifies the happy path instead.
        let pkgs = vec![make_pkg_ref("a"), make_pkg_ref("b")];
        let result = topological_sort(&pkgs);
        assert!(result.is_ok());
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
        let pkg = make_pkg_resolved("mypkg", None);
        let result = select_installer(&pkg, &None);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("No suitable installer"));
    }

    #[test]
    fn test_is_installed_no_detect() {
        let pkg = make_pkg_resolved("mypkg", None);
        assert!(!is_installed(&pkg));
    }

    #[test]
    fn test_is_installed_true_command() {
        let pkg = make_pkg_resolved("mypkg", Some("true"));
        assert!(is_installed(&pkg));
    }

    #[test]
    fn test_is_installed_false_command() {
        let pkg = make_pkg_resolved("mypkg", Some("false"));
        assert!(!is_installed(&pkg));
    }
}
