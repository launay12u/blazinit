pub mod cli;
pub mod config;
pub mod installer;
pub mod logging;
pub mod profile;
pub mod registry;

fn resolve_profile_name(profile_arg: &Option<String>) -> String {
    profile_arg
        .as_ref()
        .map_or_else(config::get_default_profile, |s| s.to_string())
}

pub fn run(cli: cli::Cli) -> Result<(), Box<dyn std::error::Error>> {
    log::debug!("bootstrapping config");
    config::bootstrap_config()?;

    match &cli.command {
        cli::Commands::Create { profile, default } => {
            log::debug!("command: create profile '{}' (set_default={})", profile, default);
            profile::create_profile(profile)?;
            if *default {
                config::set_default_profile(profile)?;
            }
        }

        cli::Commands::Delete { profile } => {
            log::debug!("command: delete profile '{}'", profile);
            profile::delete_profile(profile)?;
        }

        cli::Commands::SetDefault { profile } => {
            log::debug!("command: set-default '{}'", profile);
            config::set_default_profile(profile)?;
        }

        cli::Commands::List => {
            log::debug!("command: list profiles");
            profile::list_profiles();
        }

        cli::Commands::Show { profile } => {
            let profile_name = resolve_profile_name(profile);
            log::debug!("command: show profile '{}'", profile_name);
            profile::show_profile(&profile_name)?;
        }

        cli::Commands::Add {
            profile,
            package,
            installer,
        } => {
            let profile_name = resolve_profile_name(profile);
            log::debug!(
                "command: add package '{}' to profile '{}' (installer={:?})",
                package,
                profile_name,
                installer
            );
            profile::add_package_to_profile(
                &profile_name,
                package,
                installer.clone(),
            )?;
        }

        cli::Commands::Remove { profile, package } => {
            let profile_name = resolve_profile_name(profile);
            log::debug!("command: remove package '{}' from profile '{}'", package, profile_name);
            profile::remove_package_from_profile(&profile_name, package)?;
        }

        cli::Commands::Export { profile, file } => {
            let profile_name = resolve_profile_name(profile);
            log::debug!("command: export profile '{}' to {:?}", profile_name, file);
            profile::export_profile(&profile_name, file)?;
        }

        cli::Commands::Import { file } => {
            log::debug!("command: import profile from '{}'", file);
            profile::import_profile(file)?;
        }

        cli::Commands::Install {
            profile,
            force,
            installer,
            dry_run,
        } => {
            let profile_name = resolve_profile_name(profile);
            log::debug!(
                "command: install profile '{}' (force={}, dry_run={}, installer={:?})",
                profile_name,
                force,
                dry_run,
                installer
            );
            profile::install_profile(
                &profile_name,
                *force,
                installer,
                *dry_run,
            )?;
        }

        cli::Commands::Registry { command } => match command {
            cli::RegistryCommands::List { query } => {
                log::debug!("command: registry list (query={:?})", query);
                registry::list_packages(query)?;
            }
        },
    }

    Ok(())
}
