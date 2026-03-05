use std::io::Read;

use colored::Colorize;
use serde::Deserialize;

const GITHUB_REPO: &str = "launay12u/blazinit";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

fn current_target() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    let parts: Vec<u32> = v
        .trim_start_matches('v')
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

pub fn self_update(check_only: bool) -> Result<(), String> {
    let current = env!("CARGO_PKG_VERSION");
    let api_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    log::info!("self-update: fetching latest release from {}", api_url);

    let release: GithubRelease = ureq::get(&api_url)
        .set("User-Agent", &format!("blazinit/{}", current))
        .call()
        .map_err(|e| format!("Failed to reach GitHub: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let latest = release.tag_name.trim_start_matches('v');

    if parse_version(latest) <= parse_version(current) {
        println!("{} ({})", "Already up to date.".green(), current.cyan());
        return Ok(());
    }

    println!(
        "New version available: {} → {}",
        current.dimmed(),
        latest.green().bold()
    );

    if check_only {
        println!("Run {} to install it.", "blazinit self-update".cyan());
        return Ok(());
    }

    let target = current_target()
        .ok_or("Self-update is not supported on this platform.")?;

    let asset_name = if cfg!(windows) {
        format!("blazinit-{}.exe", target)
    } else {
        format!("blazinit-{}", target)
    };

    log::debug!("looking for asset '{}'", asset_name);

    let download_url = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .map(|a| a.browser_download_url.as_str())
        .ok_or_else(|| {
            format!(
                "No binary found for your platform ({target}). \
                 See https://github.com/{GITHUB_REPO}/releases"
            )
        })?;

    log::info!("downloading {}", download_url);
    println!("Downloading {}...", asset_name.cyan());

    let mut bytes: Vec<u8> = Vec::new();
    ureq::get(download_url)
        .call()
        .map_err(|e| format!("Download failed: {}", e))?
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read download: {}", e))?;

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot locate current executable: {}", e))?;
    let tmp = current_exe.with_extension("update.tmp");

    std::fs::write(&tmp, &bytes)
        .map_err(|e| format!("Failed to write update to disk: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp)
            .map_err(|e| format!("Failed to read temp file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    std::fs::rename(&tmp, &current_exe).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!(
            "Failed to replace binary (try running with elevated permissions): {}",
            e
        )
    })?;

    log::info!("updated binary: v{} → v{}", current, latest);
    println!("{} v{}!", "Updated to".green(), latest.green().bold());

    Ok(())
}
