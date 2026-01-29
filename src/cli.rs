use clap::{Parser, Subcommand};

use crate::profile;

#[derive(Parser)]
#[command(name = "blazinit")]
#[command(version)]
#[command(
    about = "CLI tool to manage software profiles and install all software in one command",
    long_about = "Blazinit allows you to create, modify, export/import, and install software profiles. A profile is a list of software identifiers, and you can perform operations on the whole profile at once."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Create a new profile to hold software")]
    Create {
        #[arg(help = "Name of the profile to create")]
        profile: String,
    },

    #[command(about = "Delete an existing profile")]
    Delete {
        #[arg(help = "Name of the profile to delete")]
        profile: String,
    },

    #[command(about = "List all software in a profile")]
    Show {
        #[arg(help = "Profile name to add software to", default_value_t = String::from(profile::DEFAULT_PROFILE))]
        profile: String,
    },

    #[command(about = "List all saved profiles")]
    List,

    #[command(about = "Add a software dependency to a profile")]
    Add {
        #[arg(help = "Software identifier to add")]
        software: String,
        #[arg(help = "Profile name to add software to", default_value_t = String::from(profile::DEFAULT_PROFILE))]
        profile: String,
    },

    #[command(about = "Remove a software dependency from a profile")]
    Remove {
        #[arg(help = "Software identifier to remove")]
        software: String,
        #[arg(help = "Profile name to add software to", default_value_t = String::from(profile::DEFAULT_PROFILE))]
        profile: String,
    },

    #[command(about = "List available software packages")]
    ListPackages,

    #[command(about = "Export a profile to a TOML file")]
    Export {
        #[arg(help = "Profile name to add software to", default_value_t = String::from(profile::DEFAULT_PROFILE))]
        profile: String,
        #[arg(help = "Optional file path to export to")]
        file: Option<String>,
    },

    #[command(about = "Import a profile from a TOML file")]
    Import {
        #[arg(help = "File path to import the profile from")]
        file: String,
    },

    #[command(about = "Install all software defined in a profile")]
    Install {
        #[arg(help = "Profile name to add software to", default_value_t = String::from(profile::DEFAULT_PROFILE))]
        profile: String,
        #[arg(long, help = "Print the commands that would be executed, but do not execute them")]
        dry_run: bool,
    },
}
