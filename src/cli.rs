use clap::{Parser, Subcommand};

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
    #[command(about = "Create a new profile to hold packages")]
    Create {
        #[arg(help = "Name of the profile to create")]
        profile: String,
        #[arg(
            long,
            help = "Set this profile as the default profile after creation"
        )]
        default: bool,
    },

    #[command(about = "Delete an existing profile")]
    Delete {
        #[arg(help = "Name of the profile to delete")]
        profile: String,
    },

    #[command(about = "List all packages in a profile")]
    Show {
        #[arg(
            help = "Profile name to show. Defaults to current default profile if not specified"
        )]
        profile: Option<String>,
    },

    #[command(about = "List all saved profiles")]
    List,

    #[command(about = "Add a package dependency to a profile")]
    Add {
        #[arg(help = "Package identifier to add")]
        package: String,
        #[arg(
            help = "Profile name to add package to. Defaults to current default profile if not specified"
        )]
        profile: Option<String>,
    },

    #[command(about = "Remove a package dependency from a profile")]
    Remove {
        #[arg(help = "Package identifier to remove")]
        package: String,
        #[arg(
            help = "Profile name to remove package from. Defaults to current default profile if not specified"
        )]
        profile: Option<String>,
    },

    #[command(about = "List available software packages")]
    ListPackages {
        #[arg(help = "Optional search query to filter packages")]
        query: Option<String>,
    },

    #[command(about = "Export a profile to a TOML file")]
    Export {
        #[arg(
            help = "Profile name to export. Defaults to current default profile if not specified"
        )]
        profile: Option<String>,
        #[arg(help = "Optional file path to export to")]
        file: Option<String>,
    },

    #[command(about = "Import a profile from a TOML file")]
    Import {
        #[arg(help = "File path to import the profile from")]
        file: String,
    },

    #[command(about = "Install all packages defined in a profile")]
    Install {
        #[arg(
            help = "Profile name to install. Defaults to current default profile if not specified"
        )]
        profile: Option<String>,
    },

    #[command(about = "Set the default profile")]
    SetDefault {
        #[arg(help = "Name of the profile to set as default")]
        profile: String,
    },
}
