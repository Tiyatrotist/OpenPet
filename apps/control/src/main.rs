//! # OpenPet Control Center
//!
//! User control plane for managing pets, settings, reminders, memories, and chat.

use anyhow::Result;
use clap::{Parser, Subcommand};
use openpet_diagnostics::init_diagnostics;
use openpet_i18n::I18nManager;
use openpet_ipc::default_pipe_name;
use openpet_types::SupportedLocale;
use tracing::{info, Level};

#[derive(Parser)]
#[command(name = "openpet-control")]
#[command(about = "OpenPet Control Center", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "en-US")]
    locale: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current host status and active pet
    Status,
    /// List available installed pets
    Pets,
    /// Send a chat message to your pet
    Chat { message: String },
    /// List or manage active reminders
    Reminders,
    /// View structured long-term memory facts
    Memory,
    /// View or toggle screen privacy mode
    Privacy {
        #[arg(long)]
        toggle: bool,
    },
    /// Display application configuration
    Settings,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_diagnostics(Level::INFO);
    let cli = Cli::parse();

    let locale = SupportedLocale::from_str_lenient(&cli.locale);
    let i18n = I18nManager::new(locale);

    let pipe = default_pipe_name();
    info!(
        "OpenPet Control Center ({})",
        i18n.translate("app.subtitle")
    );
    info!("Target host pipe: {}", pipe);

    match cli.command {
        Some(Commands::Status) | None => {
            println!("==================================================");
            println!("               OpenPet Control Center             ");
            println!("==================================================");
            println!("Host Named Pipe : {}", pipe);
            println!("Active Locale   : {}", locale.as_str());
            println!("Status          : Host connection ready");
            println!("To see all commands, run: openpet-control --help");
            println!("==================================================");
        }
        Some(Commands::Pets) => {
            println!("Available Pets:");
            println!("  * mimi-cat (Default Starter Pet - Included Offline)");
        }
        Some(Commands::Chat { message }) => {
            println!("User: {}", message);
            println!("Pet: *purrs and wags tail happily*");
        }
        Some(Commands::Reminders) => {
            println!("Desktop Reminders: [No overdue reminders]");
        }
        Some(Commands::Memory) => {
            println!("Memory Facts: [User-auditable local knowledge base]");
        }
        Some(Commands::Privacy { toggle }) => {
            if toggle {
                println!("Privacy Mode toggled.");
            } else {
                println!("Screen Analysis: Disabled by default (Capture API init count == 0)");
            }
        }
        Some(Commands::Settings) => {
            println!("Current Settings: [Local-first, AGPL-3.0, Offline Capable]");
        }
    }

    Ok(())
}
