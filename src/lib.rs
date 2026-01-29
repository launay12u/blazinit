pub mod cli;
pub mod config;
pub mod profile;
pub mod registry;

pub fn run(cli: cli::Cli) -> Result<(), Box<dyn std::error::Error>> {
    config::bootstrap_config()?;

    match &cli.command {
        cli::Commands::Create { profile } => profile::create_profile(profile)?,
        cli::Commands::Delete { profile } => profile::delete_profile(profile)?,
        cli::Commands::List => profile::list_profiles(),
        cli::Commands::Show { profile } => {
            println!("Show profile: {}", profile)
        }
        cli::Commands::Add { profile, software } => {
            println!("Add {} to profile {}", software, profile)
        }
        cli::Commands::ListPackages => {
            println!("List available software packages")
        }
        cli::Commands::Remove { profile, software } => {
            println!("Remove {} from profile {}", software, profile)
        }
        cli::Commands::Export { profile, file } => {
            println!("Export profile {} to {:?}", profile, file)
        }
        cli::Commands::Import { file } => {
            println!("Import profile from {:?}", file)
        }
        cli::Commands::Install { profile, dry_run } => {
            if *dry_run {
                println!("Dry run: Install profile: {}", profile);
            } else {
                println!("Install profile: {}", profile);
            }
        }
    }

    Ok(())
}
