pub mod cli;
pub mod config;
pub mod profile;
pub mod registry;

fn resolve_profile_name(profile_arg: &Option<String>) -> String {
    profile_arg
        .as_ref()
        .map_or_else(config::get_default_profile, |s| s.to_string())
}

pub fn run(cli: cli::Cli) -> Result<(), Box<dyn std::error::Error>> {
    config::bootstrap_config()?;

    match &cli.command {
        cli::Commands::Create { profile } => profile::create_profile(profile)?,

        cli::Commands::Delete { profile } => profile::delete_profile(profile)?,

        cli::Commands::SetDefault { profile } => {
            config::set_default_profile(profile)?
        }

        cli::Commands::List => profile::list_profiles(),

        cli::Commands::Show { profile } => {
            let profile_name = resolve_profile_name(profile);
            profile::show_profile(&profile_name)?
        }

        cli::Commands::Add { profile, package } => {
            let profile_name = resolve_profile_name(profile);
            profile::add_package_to_profile(&profile_name, package)?
        }

        cli::Commands::ListPackages { query } => {
            registry::list_packages(query)?;
        }
        cli::Commands::Remove { profile, package } => {
            let profile_name = resolve_profile_name(profile);
            profile::remove_package_from_profile(&profile_name, package)?
        }
        cli::Commands::Export { profile, file } => {
            let profile_name = resolve_profile_name(profile);
            profile::export_profile(&profile_name, file)?
        }
        cli::Commands::Import { file } => profile::import_profile(file)?,

        cli::Commands::Install { profile } => {
            let profile_name = resolve_profile_name(profile);
            profile::install_profile(&profile_name)?
        }
    }

    Ok(())
}
