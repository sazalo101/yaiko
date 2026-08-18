mod commands;

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "yaiko")]
#[command(author = "Yaiko Team")]
#[command(version = "0.1.0")]
#[command(about = "A modern, production-ready fullstack framework for Rust + jQuery", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Yaiko project
    Init {
        /// Project name (will create a directory with this name)
        name: String,

        /// Database type to use
        #[arg(short, long, default_value = "postgres")]
        database: String,
    },

    /// Start the development server with hot-reload
    Dev {
        /// Port to run the server on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Build the project for production
    Build {
        /// Enable release optimizations
        #[arg(short, long)]
        release: bool,
    },

    /// Run the current project once without the development watcher
    Run,

    /// Check whether the local environment is ready for Yaiko
    Doctor,

    /// Generate database migrations
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },

    /// Generate various project components
    Generate {
        #[command(subcommand)]
        component: GenerateComponent,
    },
}

#[derive(Subcommand)]
enum MigrateAction {
    /// Create a new migration
    Create {
        /// Migration name
        name: String,
    },
    /// Run pending migrations
    Run,
    /// Rollback the last migration
    Rollback,
    /// Show migration status
    Status,
}

#[derive(Subcommand)]
enum GenerateComponent {
    /// Generate a new controller
    Controller {
        /// Controller name
        name: String,
    },
    /// Generate a new model
    Model {
        /// Model name
        name: String,
    },
    /// Generate a new middleware
    Middleware {
        /// Middleware name
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    println!(
        "{}",
        "
    ██╗   ██╗ █████╗ ██╗██╗  ██╗ ██████╗ 
    ╚██╗ ██╔╝██╔══██╗██║██║ ██╔╝██╔═══██╗
     ╚████╔╝ ███████║██║█████╔╝ ██║   ██║
      ╚██╔╝  ██╔══██║██║██╔═██╗ ██║   ██║
       ██║   ██║  ██║██║██║  ██╗╚██████╔╝
       ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝ ╚═════╝ 
    "
        .cyan()
    );

    match cli.command {
        Commands::Init { name, database } => {
            commands::init::run(&name, &database).await?;
        }
        Commands::Dev { port, host } => {
            commands::dev::run(&host, port).await?;
        }
        Commands::Build { release } => {
            commands::build::run(release).await?;
        }
        Commands::Run => {
            commands::run::run().await?;
        }
        Commands::Doctor => {
            commands::doctor::run().await?;
        }
        Commands::Migrate { action } => match action {
            MigrateAction::Create { name } => {
                commands::migrate::create(&name).await?;
            }
            MigrateAction::Run => {
                commands::migrate::run().await?;
            }
            MigrateAction::Rollback => {
                commands::migrate::rollback().await?;
            }
            MigrateAction::Status => {
                commands::migrate::status().await?;
            }
        },
        Commands::Generate { component } => match component {
            GenerateComponent::Controller { name } => {
                commands::generate::controller(&name).await?;
            }
            GenerateComponent::Model { name } => {
                commands::generate::model(&name).await?;
            }
            GenerateComponent::Middleware { name } => {
                commands::generate::middleware(&name).await?;
            }
        },
    }

    Ok(())
}
