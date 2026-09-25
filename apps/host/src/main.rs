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
use openpet_platform_windows::{
    enable_per_monitor_dpi_v2, spawn_3d_dock_window, spawn_desktop_pet_window, Dock3DCommand,
    Dock3DEvent, Dock3DHandle, PetWindowCommand, SingleInstanceGuard, SingletonError, TrayAction,
};
use openpet_reminders::ReminderService;
use openpet_render::FrameScheduler;
use openpet_screen::ScreenPrivacyManager;
use openpet_storage::Database;
use openpet_types::{AppSettings, CompanionArtStyle, PetId, PetMetadata};
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
    pet_paused: Arc<AtomicBool>,
    pet_cmd_tx: std::sync::Mutex<Option<std::sync::mpsc::Sender<PetWindowCommand>>>,
    dock_3d: std::sync::Mutex<Option<Dock3DHandle>>,
}

/// Encapsulates the multi-faceted outcome of an evaluated chat query.
struct CompanionChatOutcome {
    reply: String,
    interaction: Option<openpet_types::InteractionType>,
    behavior: Option<openpet_types::BehaviorType>,
    animation: Option<(&'static str, u32)>,
    anim_3d: &'static str,
}

/// Evaluates companion chat interaction, returning user-facing response,
/// optional interaction trigger, optional behavior change, optional animation name/ticks,
/// and 3D companion animation name.
fn process_companion_chat(
    msg: &str,
    is_tr: bool,
    beh: &mut BehaviorEngine,
    memory: &MemoryService,
) -> CompanionChatOutcome {
    let lower = msg.to_lowercase();
    let (reply, interaction, behavior, animation, anim_3d) = if is_tr {
        if lower.contains("balık")
            || lower.contains("yemek")
            || lower.contains("mama")
            || lower.contains("besle")
        {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::Feed);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*mırıldanarak balığı afiyetle yer* Miyav! Nefis bir balık, çok teşekkürler! 🐟❤️"
                    .to_string(),
                Some(openpet_types::InteractionType::Feed),
                beh_type,
                Some(("eat_0", 25)),
                "purr",
            )
        } else if lower.contains("oyna") || lower.contains("top") || lower.contains("yumak") {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::Play);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*neşeyle havaya zıplar ve parıldar* Miyav! Yumakla oynamak en sevdiğim şey! ✨🐾"
                    .to_string(),
                Some(openpet_types::InteractionType::Play),
                beh_type,
                Some(("jump_0", 20)),
                "jump",
            )
        } else if lower.contains("beşlik") || lower.contains("çak") {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::HighFive);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*patisini uzatıp eline çarpar* Çaaak! Harikasın! 🖐️🐾".to_string(),
                Some(openpet_types::InteractionType::HighFive),
                beh_type,
                Some(("jump_0", 20)),
                "high_five",
            )
        } else if lower.contains("uyan") || lower.contains("kalk") {
            if beh.current_behavior() == openpet_types::BehaviorType::Sleep {
                let _ = beh.handle_interaction(openpet_types::InteractionType::SleepToggle);
            } else {
                beh.set_behavior(openpet_types::BehaviorType::Idle);
            }
            (
                "*gözlerini ovuşturup tatlı tatlı esner* Günaydın canım dostum! Ben de uyandım! ☀️🐾".to_string(),
                Some(openpet_types::InteractionType::SleepToggle),
                Some(openpet_types::BehaviorType::Idle),
                None,
                "idle",
            )
        } else if lower.contains("uyu") || lower.contains("uyku") || lower.contains("yat") {
            let _ = beh.handle_interaction(openpet_types::InteractionType::SleepToggle);
            (
                "*kocaman esneyip yanına kıvrılır* Zzz... İyi geceler tatlı insan... *mırrr* 💤"
                    .to_string(),
                Some(openpet_types::InteractionType::SleepToggle),
                Some(openpet_types::BehaviorType::Sleep),
                None,
                "sleep",
            )
        } else if lower.contains("esne") || lower.contains("gerin") {
            beh.set_behavior(openpet_types::BehaviorType::Stretch);
            (
                "*ön patilerini uzatıp sırtını gerer* Oohh... Harika bir esneme! 🐾".to_string(),
                None,
                Some(openpet_types::BehaviorType::Stretch),
                Some(("stretch_0", 24)),
                "stretch",
            )
        } else if lower.contains("şaşır") || lower.contains("korkut") || lower.contains("bö") {
            beh.set_behavior(openpet_types::BehaviorType::Surprised);
            (
                "*kuyruğu kabarır ve havaya zıplar* Viyav! Beni şaşırttın! 🙀🐾".to_string(),
                None,
                Some(openpet_types::BehaviorType::Surprised),
                Some(("surprised_0", 20)),
                "surprised",
            )
        } else if lower.contains("su") || lower.contains("içecek") || lower.contains("susad") {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::Water);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*şıkırdayan tastan lıkır lıkır taze su içer* Miyav! Su çok taze ve serinleticiydi! 💧🐾".to_string(),
                Some(openpet_types::InteractionType::Water),
                beh_type,
                Some(("drink_0", 25)),
                "purr",
            )
        } else if lower.contains("yıkan")
            || lower.contains("temizlen")
            || lower.contains("yalan")
            || lower.contains("tara")
            || lower.contains("fırça")
            || lower.contains("tüy")
        {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::Groom);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*gözlerini kısıp keyifle taranır* Mırrr... Tüylerim yumuşacık ve tertemiz oldu! ✨🐾".to_string(),
                Some(openpet_types::InteractionType::Groom),
                beh_type,
                Some(("groom_0", 25)),
                "purr",
            )
        } else if lower.contains("merhaba") || lower.contains("selam") || lower.contains("günaydın")
        {
            let _ = beh.handle_interaction(openpet_types::InteractionType::SingleClick);
            (
                "*patisini sallayarak sana sürtünür* Miyav! Merhaba canım dostum! Seninle olmak çok güzel! 🐾".to_string(),
                Some(openpet_types::InteractionType::SingleClick),
                Some(openpet_types::BehaviorType::Curious),
                None,
                "purr",
            )
        } else if lower.contains("hatırla") || lower.contains("not") {
            let _ = memory.remember("user", "shared_fact", msg, 0.9);
            let _ = beh.handle_interaction(openpet_types::InteractionType::SingleClick);
            (
                format!(
                    "*kulaklarını dikip başını sallar* Miyav! Bunu hafızama not aldım: \"{}\" 🧠🐾",
                    msg
                ),
                Some(openpet_types::InteractionType::SingleClick),
                Some(openpet_types::BehaviorType::Curious),
                None,
                "purr",
            )
        } else {
            let _ = beh.handle_interaction(openpet_types::InteractionType::SingleClick);
            (
                format!(
                    "*tatlı tatlı sana bakar ve mırıldanır* Meow! (\"{}\" mesajını dinledim!) 🐾",
                    msg
                ),
                Some(openpet_types::InteractionType::SingleClick),
                Some(openpet_types::BehaviorType::Curious),
                None,
                "purr",
            )
        }
    } else {
        if lower.contains("fish")
            || lower.contains("feed")
            || lower.contains("treat")
            || lower.contains("food")
        {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::Feed);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*purrs happily and chews the treat* Meow! That fish treat was delicious, thank you! 🐟❤️".to_string(),
                Some(openpet_types::InteractionType::Feed),
                beh_type,
                Some(("eat_0", 25)),
                "purr",
            )
        } else if lower.contains("play") || lower.contains("yarn") || lower.contains("toy") {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::Play);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*bounces high with sparkles* Meow! Playing with yarn is my favorite! ✨🐾"
                    .to_string(),
                Some(openpet_types::InteractionType::Play),
                beh_type,
                Some(("jump_0", 20)),
                "jump",
            )
        } else if lower.contains("high five") || lower.contains("highfive") {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::HighFive);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*raises paw and taps your hand* High five! You're awesome! 🖐️🐾".to_string(),
                Some(openpet_types::InteractionType::HighFive),
                beh_type,
                Some(("jump_0", 20)),
                "high_five",
            )
        } else if lower.contains("wake") || lower.contains("rise") {
            if beh.current_behavior() == openpet_types::BehaviorType::Sleep {
                let _ = beh.handle_interaction(openpet_types::InteractionType::SleepToggle);
            } else {
                beh.set_behavior(openpet_types::BehaviorType::Idle);
            }
            (
                "*rubs eyes and stretches gently* Good morning my friend! I'm wide awake now! ☀️🐾"
                    .to_string(),
                Some(openpet_types::InteractionType::SleepToggle),
                Some(openpet_types::BehaviorType::Idle),
                None,
                "idle",
            )
        } else if lower.contains("sleep") || lower.contains("nap") || lower.contains("bed") {
            let _ = beh.handle_interaction(openpet_types::InteractionType::SleepToggle);
            (
                "*yawns widely and curls into a warm ball* Zzz... Sweet dreams... *purr* 💤"
                    .to_string(),
                Some(openpet_types::InteractionType::SleepToggle),
                Some(openpet_types::BehaviorType::Sleep),
                None,
                "sleep",
            )
        } else if lower.contains("stretch") {
            beh.set_behavior(openpet_types::BehaviorType::Stretch);
            (
                "*stretches front paws and arches back* Oohh... What a refreshing stretch! 🐾"
                    .to_string(),
                None,
                Some(openpet_types::BehaviorType::Stretch),
                Some(("stretch_0", 24)),
                "stretch",
            )
        } else if lower.contains("surprise") || lower.contains("scare") || lower.contains("boo") {
            beh.set_behavior(openpet_types::BehaviorType::Surprised);
            (
                "*poofs tail and jumps back* Meow! You surprised me! 🙀🐾".to_string(),
                None,
                Some(openpet_types::BehaviorType::Surprised),
                Some(("surprised_0", 20)),
                "surprised",
            )
        } else if lower.contains("water") || lower.contains("drink") || lower.contains("thirst") {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::Water);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*happily laps fresh water from the bowl* Slurp slurp! So cool and refreshing! 💧🐾".to_string(),
                Some(openpet_types::InteractionType::Water),
                beh_type,
                Some(("drink_0", 25)),
                "purr",
            )
        } else if lower.contains("groom")
            || lower.contains("wash")
            || lower.contains("clean")
            || lower.contains("brush")
            || lower.contains("comb")
        {
            let anim_cmd = beh.handle_interaction(openpet_types::InteractionType::Groom);
            let beh_type = anim_cmd.as_ref().map(|c| c.behavior);
            (
                "*purrs deeply and leans into the soft brush* Purrr... My fur is neat and fluffy! ✨🐾".to_string(),
                Some(openpet_types::InteractionType::Groom),
                beh_type,
                Some(("groom_0", 25)),
                "purr",
            )
        } else if lower.contains("hello") || lower.contains("hi") || lower.contains("hey") {
            let _ = beh.handle_interaction(openpet_types::InteractionType::SingleClick);
            (
                "*nudges your hand affectionately* Meow! Hello best friend! Wonderful to see you! 🐾".to_string(),
                Some(openpet_types::InteractionType::SingleClick),
                Some(openpet_types::BehaviorType::Curious),
                None,
                "purr",
            )
        } else if lower.contains("remember") {
            let _ = memory.remember("user", "shared_fact", msg, 0.9);
            let _ = beh.handle_interaction(openpet_types::InteractionType::SingleClick);
            (
                format!("*perks ears up* Meow! I'll remember that: \"{}\" 🧠🐾", msg),
                Some(openpet_types::InteractionType::SingleClick),
                Some(openpet_types::BehaviorType::Curious),
                None,
                "purr",
            )
        } else {
            let _ = beh.handle_interaction(openpet_types::InteractionType::SingleClick);
            (
                format!(
                    "*purrs softly and looks up at you* Meow! (I noted: \"{}\") 🐾",
                    msg
                ),
                Some(openpet_types::InteractionType::SingleClick),
                Some(openpet_types::BehaviorType::Curious),
                None,
                "purr",
            )
        }
    };

    CompanionChatOutcome {
        reply,
        interaction,
        behavior,
        animation,
        anim_3d,
    }
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
        let official_pack_candidates = [
            std::path::PathBuf::from("packs/official/mimi-cat.openpet"),
            paths.root.join("packs/official/mimi-cat.openpet"),
        ];
        let mut installed = false;
        for pack_path in &official_pack_candidates {
            if pack_path.exists() {
                if let Ok(meta) = openpet_petpack::install_petpack_atomically(
                    pack_path,
                    &paths.packs,
                    &paths.staging,
                ) {
                    let _ = db.save_pet(&meta);
                    info!(
                        "Installed official starter pack: {} ({})",
                        meta.name, meta.id
                    );
                    installed = true;
                    break;
                }
            }
        }
        if !installed {
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
    }

    let reminders = Arc::new(ReminderService::new(db.clone()));
    let memory = Arc::new(MemoryService::new(db.clone()));
    let screen = Mutex::new(ScreenPrivacyManager::new());
    let behavior = Mutex::new(BehaviorEngine::new());
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let pet_paused = Arc::new(AtomicBool::new(false));

    let (interaction_tx, interaction_rx) = std::sync::mpsc::channel();
    let (tray_action_tx, tray_action_rx) = std::sync::mpsc::channel();
    let (chat_input_tx, chat_input_rx) = std::sync::mpsc::channel();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();

    let (dock_event_tx, dock_event_rx) = std::sync::mpsc::channel();
    let is_realistic = settings.art_style == CompanionArtStyle::Realistic;
    let dock_3d_handle = match spawn_3d_dock_window(is_realistic, dock_event_tx) {
        Ok(h) => {
            info!("3D Companion & Chat Dock initialized successfully.");
            Some(h)
        }
        Err(e) => {
            warn!("3D Companion Dock initialization failed: {}", e);
            None
        }
    };

    let state = Arc::new(HostState {
        db: db.clone(),
        behavior,
        reminders: reminders.clone(),
        memory: memory.clone(),
        screen,
        settings: Mutex::new(settings.clone()),
        shutdown_flag: shutdown_flag.clone(),
        pet_paused: pet_paused.clone(),
        pet_cmd_tx: std::sync::Mutex::new(Some(cmd_tx)),
        dock_3d: std::sync::Mutex::new(dock_3d_handle),
    });

    // 8. Spawn Native Desktop Pet Window & System Tray on UI thread
    #[cfg(windows)]
    let _pet_window_handle = spawn_desktop_pet_window(
        (0, 0),
        settings.locale,
        settings.privacy_mode,
        settings.cat_breed,
        settings.art_style,
        settings.always_on_top,
        interaction_tx,
        tray_action_tx,
        chat_input_tx,
        cmd_rx,
        shutdown_flag.clone(),
    );

    // 9. Spawn Behavior & Reminder Evaluation Tick Task (10 Hz)
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let mut scheduler = FrameScheduler::new();

        while !state_clone.shutdown_flag.load(Ordering::SeqCst) {
            interval.tick().await;

            if state_clone.pet_paused.load(Ordering::SeqCst) {
                continue;
            }

            // Behavior tick
            {
                let mut beh = state_clone.behavior.lock().await;
                if let Some(cmd) = beh.tick(0.1) {
                    scheduler.update_behavior_mode(cmd.behavior, beh.is_dragging());
                    if let Ok(guard) = state_clone.pet_cmd_tx.lock() {
                        if let Some(ref tx) = *guard {
                            let _ = tx.send(PetWindowCommand::SetBehavior(cmd.behavior));
                            if let Some(pos) = cmd.target_pos {
                                let _ = tx.send(PetWindowCommand::SetTargetPosition {
                                    x: pos.x as i32,
                                    y: pos.y as i32,
                                });
                            }
                        }
                    }
                }
            }

            // Reminder check every ~10 ticks
            let now = Utc::now();
            let _ = state_clone.reminders.evaluate_due_reminders(now);
        }
    });

    // 10. Spawn Pet Window Interaction Event Listener
    let state_interaction = state.clone();
    tokio::spawn(async move {
        while !state_interaction.shutdown_flag.load(Ordering::SeqCst) {
            while let Ok(interaction) = interaction_rx.try_recv() {
                info!("Pet interaction received by host: {:?}", interaction);
                let mut beh = state_interaction.behavior.lock().await;
                if let Some(cmd) = beh.handle_interaction(interaction.clone()) {
                    if let Ok(guard) = state_interaction.pet_cmd_tx.lock() {
                        if let Some(ref tx) = *guard {
                            let _ = tx.send(PetWindowCommand::SetBehavior(cmd.behavior));
                            let _ = tx.send(PetWindowCommand::TriggerAction(interaction));
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    // 11. Spawn Taskbar System Tray Event Listener
    let state_tray = state.clone();
    tokio::spawn(async move {
        while !state_tray.shutdown_flag.load(Ordering::SeqCst) {
            while let Ok(action) = tray_action_rx.try_recv() {
                match action {
                    TrayAction::OpenControlCenter => {
                        info!("System Tray: Requesting Control Center focus");
                    }
                    TrayAction::OpenSettings => {
                        info!("System Tray: Requesting Control Center settings");
                    }
                    TrayAction::TogglePrivacyMode => {
                        let mut settings = state_tray.settings.lock().await;
                        settings.privacy_mode = !settings.privacy_mode;
                        let priv_mode = settings.privacy_mode;
                        let _ = state_tray.db.save_settings(&settings);
                        state_tray.screen.lock().await.set_privacy_mode(priv_mode);
                        info!("System Tray: Privacy mode updated to: {}", priv_mode);
                    }
                    TrayAction::TogglePausePet => {
                        let prev = state_tray.pet_paused.load(Ordering::SeqCst);
                        let next = !prev;
                        state_tray.pet_paused.store(next, Ordering::SeqCst);
                        info!("System Tray: Pet pause toggled to: {}", next);
                        if let Ok(guard) = state_tray.pet_cmd_tx.lock() {
                            if let Some(ref tx) = *guard {
                                let _ = tx.send(PetWindowCommand::SetPaused(next));
                            }
                        }
                    }
                    TrayAction::TogglePetVisibility => {
                        info!("System Tray: Toggle pet visibility");
                    }
                    TrayAction::ToggleArtStyle => {
                        let mut settings = state_tray.settings.lock().await;
                        settings.art_style = match settings.art_style {
                            CompanionArtStyle::PixelArt => CompanionArtStyle::Realistic,
                            CompanionArtStyle::Realistic => CompanionArtStyle::PixelArt,
                        };
                        let style = settings.art_style;
                        let _ = state_tray.db.save_settings(&settings);
                        info!("System Tray: Companion art style toggled to: {:?}", style);
                        if let Ok(guard) = state_tray.pet_cmd_tx.lock() {
                            if let Some(ref tx) = *guard {
                                let _ = tx.send(PetWindowCommand::SetArtStyle(style));
                            }
                        }
                        if let Ok(guard) = state_tray.dock_3d.lock() {
                            if let Some(ref dock) = *guard {
                                let is_3d = style == CompanionArtStyle::Realistic;
                                let _ = dock.send(Dock3DCommand::Show(is_3d));
                                let _ = dock.send(Dock3DCommand::SetArtStyle(style));
                            }
                        }
                    }
                    TrayAction::Toggle3DDock => {
                        let mut settings = state_tray.settings.lock().await;
                        settings.art_style = match settings.art_style {
                            CompanionArtStyle::PixelArt => CompanionArtStyle::Realistic,
                            CompanionArtStyle::Realistic => CompanionArtStyle::PixelArt,
                        };
                        let style = settings.art_style;
                        let _ = state_tray.db.save_settings(&settings);
                        info!(
                            "TrayAction: Toggled 3D Companion Dock, new style: {:?}",
                            style
                        );
                        if let Ok(guard) = state_tray.pet_cmd_tx.lock() {
                            if let Some(ref tx) = *guard {
                                let _ = tx.send(PetWindowCommand::SetArtStyle(style));
                            }
                        }
                        if let Ok(guard) = state_tray.dock_3d.lock() {
                            if let Some(ref dock) = *guard {
                                let is_3d = style == CompanionArtStyle::Realistic;
                                let _ = dock.send(Dock3DCommand::Show(is_3d));
                                let _ = dock.send(Dock3DCommand::SetArtStyle(style));
                            }
                        }
                    }
                    TrayAction::ExitApplication => {
                        info!("System Tray: Exit application requested. Initiating graceful shutdown.");
                        state_tray.shutdown_flag.store(true, Ordering::SeqCst);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // 12. Spawn Floating Chat Bubble Event Listener
    let state_chat = state.clone();
    tokio::spawn(async move {
        while !state_chat.shutdown_flag.load(Ordering::SeqCst) {
            while let Ok(msg) = chat_input_rx.try_recv() {
                info!("Chat input from floating bubble: {}", msg);

                let is_tr = {
                    let settings = state_chat.settings.lock().await;
                    settings.locale == openpet_types::SupportedLocale::TrTr
                };

                let outcome = {
                    let mut beh = state_chat.behavior.lock().await;
                    process_companion_chat(&msg, is_tr, &mut beh, &state_chat.memory)
                };

                // Synchronize pet window
                if let Ok(guard) = state_chat.pet_cmd_tx.lock() {
                    if let Some(ref tx) = *guard {
                        if let Some(b) = outcome.behavior {
                            let _ = tx.send(PetWindowCommand::SetBehavior(b));
                        }
                        if let Some(i) = outcome.interaction {
                            let _ = tx.send(PetWindowCommand::TriggerAction(i));
                        }
                        if let Some((anim_name, ticks)) = outcome.animation {
                            let _ = tx.send(PetWindowCommand::PlayAnimation {
                                name: anim_name.to_string(),
                                duration_ticks: ticks,
                            });
                        }
                        let _ = tx.send(PetWindowCommand::AddChatMessage {
                            sender: "Mimi".to_string(),
                            text: outcome.reply.clone(),
                        });
                    }
                }

                // Forward to 3D dock
                if let Ok(guard) = state_chat.dock_3d.lock() {
                    if let Some(ref dock) = *guard {
                        dock.send_chat_response(&outcome.reply);
                        dock.trigger_anim(outcome.anim_3d);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    // 13. Spawn 3D Animated Companion Dock Event Listener
    let state_dock = state.clone();
    tokio::spawn(async move {
        while !state_dock.shutdown_flag.load(Ordering::SeqCst) {
            while let Ok(event) = dock_event_rx.try_recv() {
                match event {
                    Dock3DEvent::SubmitChat(text) => {
                        info!("3D Dock prompt submission: {}", text);
                        let is_tr = {
                            let settings = state_dock.settings.lock().await;
                            settings.locale == openpet_types::SupportedLocale::TrTr
                        };
                        let outcome = {
                            let mut beh = state_dock.behavior.lock().await;
                            process_companion_chat(&text, is_tr, &mut beh, &state_dock.memory)
                        };

                        // Send reply to 3D dock speech bubble and trigger 3D animation
                        if let Ok(guard) = state_dock.dock_3d.lock() {
                            if let Some(ref dock) = *guard {
                                dock.send_chat_response(&outcome.reply);
                                dock.trigger_anim(outcome.anim_3d);
                            }
                        }

                        // Synchronize with desktop pet window
                        if let Ok(guard) = state_dock.pet_cmd_tx.lock() {
                            if let Some(ref tx) = *guard {
                                if let Some(b) = outcome.behavior {
                                    let _ = tx.send(PetWindowCommand::SetBehavior(b));
                                }
                                if let Some(i) = outcome.interaction {
                                    let _ = tx.send(PetWindowCommand::TriggerAction(i));
                                }
                                if let Some((anim_name, ticks)) = outcome.animation {
                                    let _ = tx.send(PetWindowCommand::PlayAnimation {
                                        name: anim_name.to_string(),
                                        duration_ticks: ticks,
                                    });
                                }
                                let _ = tx.send(PetWindowCommand::AddChatMessage {
                                    sender: "Oreo".to_string(),
                                    text: outcome.reply,
                                });
                            }
                        }
                    }
                    Dock3DEvent::Interact(action) => {
                        info!("3D Dock interaction: {}", action);
                        let (interaction, anim_3d) = match action.as_str() {
                            "feed" => (openpet_types::InteractionType::Feed, "purr"),
                            "water" => (openpet_types::InteractionType::Water, "purr"),
                            "play" => (openpet_types::InteractionType::Play, "jump"),
                            "highfive" => (openpet_types::InteractionType::HighFive, "high_five"),
                            "sleep" => (openpet_types::InteractionType::SleepToggle, "sleep"),
                            _ => (openpet_types::InteractionType::SingleClick, "purr"),
                        };
                        let mut beh = state_dock.behavior.lock().await;
                        if let Some(cmd) = beh.handle_interaction(interaction.clone()) {
                            if let Ok(guard) = state_dock.pet_cmd_tx.lock() {
                                if let Some(ref tx) = *guard {
                                    let _ = tx.send(PetWindowCommand::SetBehavior(cmd.behavior));
                                    let _ = tx.send(PetWindowCommand::TriggerAction(interaction));
                                }
                            }
                        }
                        if let Ok(guard) = state_dock.dock_3d.lock() {
                            if let Some(ref dock) = *guard {
                                dock.trigger_anim(anim_3d);
                            }
                        }
                    }
                    Dock3DEvent::ToggleArtStyle => {
                        let mut settings = state_dock.settings.lock().await;
                        settings.art_style = match settings.art_style {
                            CompanionArtStyle::PixelArt => CompanionArtStyle::Realistic,
                            CompanionArtStyle::Realistic => CompanionArtStyle::PixelArt,
                        };
                        let style = settings.art_style;
                        let _ = state_dock.db.save_settings(&settings);
                        info!("3D Dock: Art style toggled to: {:?}", style);
                        if let Ok(guard) = state_dock.pet_cmd_tx.lock() {
                            if let Some(ref tx) = *guard {
                                let _ = tx.send(PetWindowCommand::SetArtStyle(style));
                            }
                        }
                        if let Ok(guard) = state_dock.dock_3d.lock() {
                            if let Some(ref dock) = *guard {
                                let is_3d = style == CompanionArtStyle::Realistic;
                                let _ = dock.send(Dock3DCommand::Show(is_3d));
                                let _ = dock.send(Dock3DCommand::SetArtStyle(style));
                            }
                        }
                    }
                    Dock3DEvent::Closed => {
                        info!("3D Companion Dock closed.");
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    info!("Behavior engine, desktop pet, system tray, and reminder scheduler active.");

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
                if let Some(cmd) = beh.handle_interaction(interaction.clone()) {
                    if let Ok(guard) = self.pet_cmd_tx.lock() {
                        if let Some(ref tx) = *guard {
                            let _ = tx.send(PetWindowCommand::SetBehavior(cmd.behavior));
                            let _ = tx.send(PetWindowCommand::TriggerAction(interaction));
                        }
                    }
                }
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
                if let Ok(guard) = self.pet_cmd_tx.lock() {
                    if let Some(ref tx) = *guard {
                        let _ =
                            tx.send(PetWindowCommand::SetPrivacyMode(new_settings.privacy_mode));
                        let _ = tx.send(PetWindowCommand::SetLocale(new_settings.locale));
                        let _ = tx.send(PetWindowCommand::SetBreed(new_settings.cat_breed));
                        let _ = tx.send(PetWindowCommand::SetArtStyle(new_settings.art_style));
                        let _ =
                            tx.send(PetWindowCommand::SetAlwaysOnTop(new_settings.always_on_top));
                    }
                }
                if let Ok(guard) = self.dock_3d.lock() {
                    if let Some(ref dock) = *guard {
                        let is_3d = new_settings.art_style == CompanionArtStyle::Realistic;
                        let _ = dock.send(Dock3DCommand::Show(is_3d));
                        let _ = dock.send(Dock3DCommand::SetArtStyle(new_settings.art_style));
                    }
                }
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
            IpcRequest::ToggleReminder(id) => match self.reminders.toggle_reminder(id) {
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
                if let Ok(guard) = self.pet_cmd_tx.lock() {
                    if let Some(ref tx) = *guard {
                        let _ = tx.send(PetWindowCommand::SetPrivacyMode(active));
                    }
                }
                IpcResponse::Ack
            }
            IpcRequest::SendChatMessage {
                conversation_id,
                content,
            } => {
                let mut beh = self.behavior.lock().await;
                if let Some(cmd) =
                    beh.handle_interaction(openpet_types::InteractionType::SingleClick)
                {
                    if let Ok(guard) = self.pet_cmd_tx.lock() {
                        if let Some(ref tx) = *guard {
                            let _ = tx.send(PetWindowCommand::SetBehavior(cmd.behavior));
                        }
                    }
                }
                let is_tr = {
                    let settings = self.settings.lock().await;
                    settings.locale == openpet_types::SupportedLocale::TrTr
                };
                let reply_text = if is_tr {
                    format!(
                        "*mırıldanarak sana bakar* Meow! (\"{}\" mesajını duydum!)",
                        content
                    )
                } else {
                    format!(
                        "*purrs softly and nudges your hand* Meow! (I heard: \"{}\")",
                        content
                    )
                };
                let response_msg = ChatMessage::assistant(conversation_id, reply_text);
                IpcResponse::ChatMessage(response_msg)
            }
            IpcRequest::Ping => IpcResponse::Pong,
            IpcRequest::Shutdown => {
                if let Ok(guard) = self.pet_cmd_tx.lock() {
                    if let Some(ref tx) = *guard {
                        let _ = tx.send(PetWindowCommand::Close);
                    }
                }
                if let Ok(guard) = self.dock_3d.lock() {
                    if let Some(ref dock) = *guard {
                        let _ = dock.send(Dock3DCommand::Close);
                    }
                }
                self.shutdown_flag.store(true, Ordering::SeqCst);
                IpcResponse::Ack
            }
            _ => IpcResponse::Ack,
        }
    }
}
