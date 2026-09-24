//! # OpenPet Control Center
//!
//! User control plane for managing pets, settings, reminders, memories, and chat.

use anyhow::Result;
use clap::{Parser, Subcommand};
use openpet_diagnostics::init_diagnostics;
use openpet_i18n::I18nManager;
use openpet_ipc::{default_pipe_name, IpcClient};
use openpet_types::{IpcRequest, IpcResponse, SupportedLocale};
use tracing::{info, Level};
use uuid::Uuid;

pub mod gui;

#[derive(Parser)]
#[command(name = "openpet-control")]
#[command(about = "OpenPet Control Center", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "en-US")]
    locale: String,

    /// Force launching in Graphical UI mode
    #[arg(long)]
    gui: bool,

    /// Force launching directly into the Settings tab
    #[arg(long)]
    settings: bool,

    /// Initial tab to display in GUI (status, chat, reminders, privacy, settings)
    #[arg(long)]
    tab: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the interactive Graphical Control Center window
    Gui,
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

fn main() -> Result<()> {
    init_diagnostics(Level::INFO);
    let cli = Cli::parse();

    let locale = SupportedLocale::from_str_lenient(&cli.locale);
    let i18n = I18nManager::new(locale);

    let initial_tab = if cli.settings {
        gui::TAB_SETTINGS
    } else if let Some(ref t) = cli.tab {
        match t.to_lowercase().as_str() {
            "chat" => gui::TAB_CHAT,
            "reminders" | "reminder" => gui::TAB_REMINDERS,
            "privacy" => gui::TAB_PRIVACY,
            "settings" | "setting" => gui::TAB_SETTINGS,
            _ => gui::TAB_STATUS,
        }
    } else {
        gui::TAB_STATUS
    };

    // If no CLI subcommand is provided, or --gui/--settings/--tab/Commands::Gui is requested, launch the rich GUI window!
    if cli.command.is_none()
        || cli.gui
        || cli.settings
        || cli.tab.is_some()
        || matches!(cli.command, Some(Commands::Gui))
    {
        info!("Launching OpenPet Graphical Control Center window...");
        gui::run_control_center_gui(locale, initial_tab);
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let pipe = default_pipe_name();
        info!(
            "OpenPet Control Center ({})",
            i18n.translate("app.subtitle")
        );
        info!("Target host pipe: {}", pipe);

        let mut client = IpcClient::connect(&pipe).await.ok();
        let is_connected = client.is_some();

        match cli.command {
            Some(Commands::Gui) => unreachable!(),
            Some(Commands::Status) | None => {
            println!("==================================================");
            println!("               OpenPet Control Center             ");
            println!("==================================================");
            println!("Host Named Pipe : {}", pipe);
            println!("Active Locale   : {}", locale.as_str());
            println!(
                "Host Status     : {}",
                if is_connected {
                    "ONLINE (Connected via Named Pipe)"
                } else {
                    "OFFLINE (Start openpet-host to activate background engine)"
                }
            );

            if let Some(ref mut c) = client {
                if let Ok(IpcResponse::ActivePet(Some(pet))) =
                    c.request(&IpcRequest::GetActivePet).await
                {
                    println!("Active Pet      : {} ({})", pet.name, pet.id);
                }
                if let Ok(IpcResponse::PetState(state)) = c.request(&IpcRequest::GetPetState).await
                {
                    println!(
                        "Pet State       : Mood: {:.2}, Energy: {:.2}, Circadian: {:?}",
                        state.mood, state.energy, state.circadian_phase
                    );
                }
            } else {
                println!("Active Pet      : mimi-cat (Default)");
            }
            println!("To see all commands, run: openpet-control --help");
            println!("==================================================");
        }
        Some(Commands::Pets) => {
            println!("--- Available Pets ---");
            if let Some(ref mut c) = client {
                match c.request(&IpcRequest::ListPets).await {
                    Ok(IpcResponse::Pets(pets)) => {
                        for p in pets {
                            println!("* {} (ID: {}) by {}", p.name, p.id, p.author);
                            println!("  Description: {}", p.description);
                            println!("  License: {}", p.license);
                        }
                    }
                    Ok(resp) => println!("Unexpected host response: {:?}", resp),
                    Err(e) => println!("Error listing pets from host: {}", e),
                }
            } else {
                println!("  * mimi-cat (Default Starter Pet - Included Offline)");
                println!(
                    "    [Notice: Run 'openpet-host' to view all live database installations]"
                );
            }
        }
        Some(Commands::Chat { message }) => {
            println!("User: {}", message);
            if let Some(ref mut c) = client {
                let req = IpcRequest::SendChatMessage {
                    conversation_id: Uuid::new_v4(),
                    content: message,
                };
                match c.request(&req).await {
                    Ok(IpcResponse::ChatMessage(chat_msg)) => {
                        println!("Pet: {}", chat_msg.content);
                    }
                    Ok(resp) => println!("Unexpected host response: {:?}", resp),
                    Err(e) => println!("Error sending chat to host: {}", e),
                }
            } else {
                println!("Pet: *purrs and wags tail happily*");
                println!("    [Notice: Start 'openpet-host' for live AI and interactive companion responses]");
            }
        }
        Some(Commands::Reminders) => {
            println!("--- Desktop Reminders ---");
            if let Some(ref mut c) = client {
                match c.request(&IpcRequest::ListReminders).await {
                    Ok(IpcResponse::Reminders(rems)) => {
                        if rems.is_empty() {
                            println!("No reminders scheduled.");
                        } else {
                            for r in rems {
                                println!(
                                    "* [{}] {} - Schedule: {} (Recurrence: {:?}, Enabled: {})",
                                    r.id, r.title, r.schedule, r.recurrence, r.enabled
                                );
                            }
                        }
                    }
                    Ok(resp) => println!("Unexpected response: {:?}", resp),
                    Err(e) => println!("Error querying reminders: {}", e),
                }
            } else {
                println!("Desktop Reminders: [No overdue reminders]");
                println!("    [Notice: Start 'openpet-host' to view live reminder schedule]");
            }
        }
        Some(Commands::Memory) => {
            println!("--- Memory Facts (Knowledge Base) ---");
            if let Some(ref mut c) = client {
                match c.request(&IpcRequest::ListMemories).await {
                    Ok(IpcResponse::Memories(mems)) => {
                        if mems.is_empty() {
                            println!("No facts stored yet.");
                        } else {
                            for m in mems {
                                println!(
                                    "* {} (Confidence: {:.2}, Locked: {})",
                                    m.display_summary(),
                                    m.confidence,
                                    m.user_locked
                                );
                            }
                        }
                    }
                    Ok(resp) => println!("Unexpected response: {:?}", resp),
                    Err(e) => println!("Error querying memories: {}", e),
                }
            } else {
                println!("Memory Facts: [User-auditable local knowledge base]");
                println!("    [Notice: Start 'openpet-host' to view live memory database]");
            }
        }
        Some(Commands::Privacy { toggle }) => {
            if toggle {
                if let Some(ref mut c) = client {
                    match c.request(&IpcRequest::SetPrivacyMode(true)).await {
                        Ok(IpcResponse::Ack) => {
                            println!("SUCCESS: Privacy Mode enabled on OpenPet Host.");
                        }
                        Ok(resp) => println!("Unexpected response: {:?}", resp),
                        Err(e) => println!("Error setting privacy mode on host: {}", e),
                    }
                } else {
                    println!("Privacy Mode toggled (local simulation).");
                }
            } else if let Some(ref mut c) = client {
                if let Ok(IpcResponse::Settings(s)) = c.request(&IpcRequest::GetSettings).await {
                    println!(
                        "Screen Analysis: {}",
                        if s.screen_analysis_enabled {
                            "Enabled (Opt-in)"
                        } else {
                            "Disabled (Zero capture calls)"
                        }
                    );
                    println!(
                        "Privacy Mode   : {}",
                        if s.privacy_mode { "ACTIVE" } else { "Inactive" }
                    );
                }
            } else {
                println!("Screen Analysis: Disabled by default (Capture API init count == 0)");
            }
        }
        Some(Commands::Settings) => {
            println!("--- OpenPet Configuration ---");
            if let Some(ref mut c) = client {
                match c.request(&IpcRequest::GetSettings).await {
                    Ok(IpcResponse::Settings(s)) => {
                        println!("Active Pet ID         : {}", s.active_pet_id);
                        println!("Locale                : {:?}", s.locale);
                        println!("Launch On Startup     : {}", s.launch_on_startup);
                        println!("Always On Top         : {}", s.always_on_top);
                        println!("Privacy Mode          : {}", s.privacy_mode);
                        println!("Screen Analysis       : {}", s.screen_analysis_enabled);
                        println!("Animation Quality     : {:?}", s.animation_quality);
                    }
                    Ok(resp) => println!("Unexpected response: {:?}", resp),
                    Err(e) => println!("Error querying settings: {}", e),
                }
            } else {
                println!("Current Settings: [Local-first, AGPL-3.0, Offline Capable]");
                println!("    [Notice: Start 'openpet-host' to inspect runtime settings]");
            }
        }
    }
    Ok(())
})
}
