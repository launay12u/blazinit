mod cli;
mod profile;

use clap::Parser;
use colored::*;

use crate::cli::Cli;

fn main() {
    profile::ensure_default_profile()
        .expect("Failed to create default profile");

    let cli = Cli::parse();

    match &cli.command {
        crate::cli::Commands::Create { profile } => {
            match profile::create_profile(profile) {
                Ok(_) => println!(
                    "{}",
                    format!("Profile '{}' created", profile).green()
                ),
                Err(e) => eprintln!("{}", e.red()),
            }
        }
        crate::cli::Commands::Delete { profile } => {
            match profile::delete_profile(profile) {
                Ok(_) => println!(
                    "{}",
                    format!("Profile '{}' removed", profile).green()
                ),
                Err(e) => eprintln!("{}", e.red()),
            }
        }
        crate::cli::Commands::List => {
            profile::list_profiles();
        }
        crate::cli::Commands::Show { profile } => {
            println!("Show profile: {}", profile);
        }
        crate::cli::Commands::Add { profile, software } => {
            println!("Add {} to profile {}", software, profile);
        }
        crate::cli::Commands::ListPackages => {
            println!("List available software packages");
        }
        crate::cli::Commands::Remove { profile, software } => {
            println!("Remove {} from profile {}", software, profile);
        }
        crate::cli::Commands::Export { profile, file } => {
            println!("Export profile {} to {:?}", profile, file);
        }
        crate::cli::Commands::Import { file } => {
            println!("Import profile from {:?}", file);
        }
        crate::cli::Commands::Install { profile } => {
            println!("Install profile: {}", profile);
        }
    }
}
