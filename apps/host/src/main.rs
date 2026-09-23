//! # OpenPet Host Executable
//!
//! Primary runtime owning the database, behavior engine, pet window lifecycle, and IPC server.

use anyhow::Result;
use chrono::Utc;
use openpet_behavior::BehaviorEngine;
use openpet_core::AppPaths;
use openpet_diagnostics::init_diagnostics;
use openpet_ipc::{default_pipe_name, IpcServer};
use openpet_memory::MemoryService;
use openpet_platform_windows::{enable_per_monitor_dpi_v2, SingleInstanceGuard, SingletonError};
use openpet_reminders::ReminderService;
use openpet_render::FrameScheduler;
use openpet_screen::ScreenPrivacyManager;
use openpet_storage::Database;
use openpet_types::{AppSettings, PetId, PetMetadata};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info, warn, Level};

#[allow(dead_code)]
struct HostState {
    db: Arc<Database>,
    behavior: Mutex<BehaviorEngine>,
    reminders: Arc<ReminderService>,
    memory: Arc<MemoryService>,
    screen: Mutex<ScreenPrivacyManager>,
    settings: Mutex<AppSettings>,
    shutdown_flag: Arc<AtomicBool>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize Structured Diagnostics & Zero-Telemetry Logging
    init_diagnostics(Level::INFO);
    info!("Starting OpenPet Host v{}", env!("CARGO_PKG_VERSION"));

    // 2. Configure Windows DPI Awareness V2
    if enable_per_monitor_dpi_v2() {
        info!("Per-Monitor DPI Awareness V2 enabled successfully.");
    }

    // 3. Acquire Single-Instance Mutex
    let _instance_guard = match SingleInstanceGuard::acquire("OpenPet_Host_Singleton_Mutex") {
        Ok(guard) => guard,
        Err(SingletonError::AlreadyRunning) => {
            warn!(
                "Existing OpenPet Host detected. Forwarding focus to Control Center and exiting."
            );
            return Ok(());
        }
        Err(e) => {
            error!("Failed acquiring singleton mutex: {:?}", e);
            return Err(anyhow::anyhow!("Singleton acquisition failure: {:?}", e));
        }
    };

    // 4. Ensure Application Directories
    let paths = AppPaths::default_windows_paths();
    paths.ensure_all_dirs()?;
    paths.cleanup_orphaned_staging()?;
    info!("App data root initialized at: {:?}", paths.root);

    // 5. Open Database and Run Migrations
    let db_path = paths.database_file();
    let db = Arc::new(Database::open(&db_path)?);
    info!("Database initialized and verified at: {:?}", db_path);

    // 6. Load Settings or Seed Defaults
    let settings = db.load_settings()?.unwrap_or_else(|| {
        let def = AppSettings::default();
        let _ = db.save_settings(&def);
        def
    });

    // 7. Seed Official Default Pet if Empty
    let pets = db.list_pets()?;
    if pets.is_empty() {
        let sample = PetMetadata {
            id: PetId::default_pet(),
            name: "Mimi the Cat".to_string(),
            version: "1.0.0".to_string(),
            author: "Tiyatrotist".to_string(),
            description: "Default playful companion cat".to_string(),
            license: "AGPL-3.0".to_string(),
            homepage: None,
            created_with: None,
            source_provenance: Some("OpenPet Official Assets".to_string()),
            minimum_openpet_version: "0.1.0".to_string(),
            tags: vec![
                "feline".to_string(),
                "cute".to_string(),
                "starter".to_string(),
            ],
        };
        db.save_pet(&sample)?;
        info!("Seeded default official pet: {}", sample.name);
    }

    let reminders = Arc::new(ReminderService::new(db.clone()));
    let memory = Arc::new(MemoryService::new(db.clone()));
    let screen = Mutex::new(ScreenPrivacyManager::new());
    let behavior = Mutex::new(BehaviorEngine::new());
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    let state = Arc::new(HostState {
        db: db.clone(),
        behavior,
        reminders: reminders.clone(),
        memory: memory.clone(),
        screen,
        settings: Mutex::new(settings),
        shutdown_flag: shutdown_flag.clone(),
    });

    // 8. Spawn Behavior & Reminder Evaluation Tick Task (10 Hz)
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let mut scheduler = FrameScheduler::new();

        while !state_clone.shutdown_flag.load(Ordering::SeqCst) {
            interval.tick().await;

            // Behavior tick
            {
                let mut beh = state_clone.behavior.lock().await;
                if let Some(cmd) = beh.tick(0.1) {
                    scheduler.update_behavior_mode(cmd.behavior, beh.is_dragging());
                }
            }

            // Reminder check every ~10 ticks
            let now = Utc::now();
            let _ = state_clone.reminders.evaluate_due_reminders(now);
        }
    });

    info!("Behavior engine and reminder scheduler active.");

    // 9. Host Named Pipe IPC Server Loop
    let pipe_name = default_pipe_name();
    info!("Starting IPC server on named pipe: {}", pipe_name);
    let state_ipc = state.clone();
    let shutdown_ipc = shutdown_flag.clone();
    let pipe_name_clone = pipe_name.clone();
    tokio::spawn(async move {
        if let Err(e) = IpcServer::run(pipe_name_clone, state_ipc, shutdown_ipc).await {
            error!("IPC server error: {}", e);
        }
    });

    // Run until shutdown signal
    let shutdown_signal = shutdown_flag.clone();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received. Initiating graceful shutdown...");
        }
        _ = async {
            while !shutdown_signal.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        } => {
            info!("Host shutdown signaled via IPC.");
        }
    }

    info!("OpenPet Host cleanly terminated.");
    Ok(())
}

#[async_trait::async_trait]
impl openpet_ipc::IpcRequestHandler for HostState {
    async fn handle_request(
        &self,
        request: openpet_types::IpcRequest,
    ) -> openpet_types::IpcResponse {
        use openpet_ipc::IPC_PROTOCOL_VERSION;
        use openpet_types::{ChatMessage, IpcRequest, IpcResponse, ServerHello};

        match request {
            IpcRequest::Handshake(_) => IpcResponse::Handshake(ServerHello {
                protocol_version: IPC_PROTOCOL_VERSION,
                host_version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities: vec![
                    "pets".into(),
                    "settings".into(),
                    "reminders".into(),
                    "memory".into(),
                    "chat".into(),
                    "privacy".into(),
                ],
            }),
            IpcRequest::ListPets => match self.db.list_pets() {
                Ok(pets) => IpcResponse::Pets(pets),
                Err(e) => IpcResponse::Error(e.to_string()),
            },
            IpcRequest::GetActivePet => {
                let current_id = self.settings.lock().await.active_pet_id.clone();
                match self.db.list_pets() {
                    Ok(pets) => {
                        let active = pets.into_iter().find(|p| p.id == current_id);
                        IpcResponse::ActivePet(active)
                    }
                    Err(e) => IpcResponse::Error(e.to_string()),
                }
            }
            IpcRequest::SetActivePet(pet_id) => {
                let mut settings = self.settings.lock().await;
                settings.active_pet_id = pet_id;
                let _ = self.db.save_settings(&settings);
                IpcResponse::Ack
            }
            IpcRequest::GetPetState => {
                let beh = self.behavior.lock().await;
                IpcResponse::PetState(beh.state().clone())
            }
            IpcRequest::InteractPet(interaction) => {
                let mut beh = self.behavior.lock().await;
                beh.handle_interaction(interaction);
                IpcResponse::Ack
            }
            IpcRequest::GetSettings => {
                let settings = self.settings.lock().await;
                IpcResponse::Settings(settings.clone())
            }
            IpcRequest::UpdateSettings(new_settings) => {
                let mut settings = self.settings.lock().await;
                *settings = new_settings.clone();
                let _ = self.db.save_settings(&new_settings);
                IpcResponse::Ack
            }
            IpcRequest::ListMemories => match self.memory.list_all() {
                Ok(memories) => IpcResponse::Memories(memories),
                Err(e) => IpcResponse::Error(e.to_string()),
            },
            IpcRequest::CreateMemory {
                subject,
                predicate,
                object,
                confidence,
            } => match self.memory.remember(subject, predicate, object, confidence) {
                Ok(_) => IpcResponse::Ack,
                Err(e) => IpcResponse::Error(e.to_string()),
            },
            IpcRequest::DeleteMemory(id) => match self.memory.forget(id) {
                Ok(_) => IpcResponse::Ack,
                Err(e) => IpcResponse::Error(e.to_string()),
            },
            IpcRequest::SetMemoryLocked { id, locked } => {
                match self.memory.set_locked(id, locked) {
                    Ok(_) => IpcResponse::Ack,
                    Err(e) => IpcResponse::Error(e.to_string()),
                }
            }
            IpcRequest::ListReminders => match self.reminders.list_reminders() {
                Ok(rems) => IpcResponse::Reminders(rems),
                Err(e) => IpcResponse::Error(e.to_string()),
            },
            IpcRequest::CreateReminder {
                title,
                body,
                schedule_utc,
                recurrence,
            } => {
                match self
                    .reminders
                    .schedule_reminder(title, body, schedule_utc, recurrence)
                {
                    Ok(_) => IpcResponse::Ack,
                    Err(e) => IpcResponse::Error(e.to_string()),
                }
            }
            IpcRequest::DeleteReminder(id) => match self.reminders.cancel_reminder(id) {
                Ok(_) => IpcResponse::Ack,
                Err(e) => IpcResponse::Error(e.to_string()),
            },
            IpcRequest::SetPrivacyMode(active) => {
                {
                    let mut screen = self.screen.lock().await;
                    screen.set_privacy_mode(active);
                }
                {
                    let mut settings = self.settings.lock().await;
                    settings.privacy_mode = active;
                    let _ = self.db.save_settings(&settings);
                }
                IpcResponse::Ack
            }
            IpcRequest::SendChatMessage {
                conversation_id,
                content,
            } => {
                let mut beh = self.behavior.lock().await;
                beh.handle_interaction(openpet_types::InteractionType::SingleClick);
                let response_msg = ChatMessage::assistant(
                    conversation_id,
                    format!(
                        "*meows softly and nudges your hand* I heard: \"{}\"",
                        content
                    ),
                );
                IpcResponse::ChatMessage(response_msg)
            }
            IpcRequest::Ping => IpcResponse::Pong,
            IpcRequest::Shutdown => {
                self.shutdown_flag.store(true, Ordering::SeqCst);
                IpcResponse::Ack
            }
            _ => IpcResponse::Ack,
        }
    }
}
