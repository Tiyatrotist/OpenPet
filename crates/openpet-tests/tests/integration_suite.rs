use chrono::{Duration, Utc};
use openpet_behavior::BehaviorEngine;
use openpet_memory::MemoryService;
use openpet_reminders::ReminderService;
use openpet_storage::Database;
use openpet_types::{AppSettings, InteractionType, RecurrenceRule, ScreenPosition};
use std::sync::Arc;

#[test]
fn test_end_to_end_behavior_and_persistence() {
    let db = Arc::new(Database::open_in_memory().unwrap());

    // 1. Settings persistence
    let settings = AppSettings {
        launch_on_startup: true,
        ..Default::default()
    };
    db.save_settings(&settings).unwrap();

    let loaded = db.load_settings().unwrap().expect("Settings should load");
    assert!(loaded.launch_on_startup);

    // 2. Behavior engine deterministic interactions
    let mut engine = BehaviorEngine::new_with_seed(42);

    // Drag sequence
    engine.handle_interaction(InteractionType::DragStart { x: 100.0, y: 200.0 });
    assert!(engine.is_dragging());
    assert_eq!(engine.position(), ScreenPosition::new(100.0, 200.0));

    // Petting increases bond and mood
    let initial_bond = engine.state().bond;
    engine.handle_interaction(InteractionType::Petting { intensity: 1.0 });
    assert!(engine.state().bond > initial_bond);
}

#[test]
fn test_end_to_end_memory_and_reminders() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let memory = MemoryService::new(db.clone());
    let reminders = ReminderService::new(db.clone());

    // Memory operations
    memory
        .remember("Owner", "Favorite drink is", "Espresso", 0.9)
        .unwrap();
    let facts = memory.search("Espresso").unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].subject, "owner");

    // Reminder operations
    let due_time = Utc::now() - Duration::minutes(5); // already due
    let reminder = reminders
        .schedule_reminder(
            "Water plants",
            "Front porch",
            due_time,
            RecurrenceRule::Daily,
        )
        .unwrap();

    let fired = reminders.evaluate_due_reminders(Utc::now()).unwrap();
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].id, reminder.id);
    // Recurrence should advance schedule by 1 day
    assert!(fired[0].schedule > Utc::now());
}

#[tokio::test]
#[cfg(windows)]
async fn test_end_to_end_ipc_control_host_communication() {
    use async_trait::async_trait;
    use openpet_ipc::{IpcClient, IpcRequestHandler, IpcServer, IPC_PROTOCOL_VERSION};
    use openpet_types::{ChatMessage, IpcRequest, IpcResponse, ServerHello};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct MockHostHandler {
        db: Arc<Database>,
    }

    #[async_trait]
    impl IpcRequestHandler for MockHostHandler {
        async fn handle_request(&self, request: IpcRequest) -> IpcResponse {
            match request {
                IpcRequest::Handshake(_) => IpcResponse::Handshake(ServerHello {
                    protocol_version: IPC_PROTOCOL_VERSION,
                    host_version: "0.1.0-test".to_string(),
                    capabilities: vec!["pets".into(), "settings".into()],
                }),
                IpcRequest::ListPets => match self.db.list_pets() {
                    Ok(pets) => IpcResponse::Pets(pets),
                    Err(e) => IpcResponse::Error(e.to_string()),
                },
                IpcRequest::SendChatMessage {
                    conversation_id,
                    content,
                } => IpcResponse::ChatMessage(ChatMessage::assistant(
                    conversation_id,
                    format!("Echo: {}", content),
                )),
                IpcRequest::Ping => IpcResponse::Pong,
                _ => IpcResponse::Ack,
            }
        }
    }

    let db = Arc::new(Database::open_in_memory().unwrap());
    let sample = openpet_types::PetMetadata {
        id: openpet_types::PetId::new("mimi-cat"),
        name: "Mimi".into(),
        version: "1.0.0".into(),
        author: "Tiyatrotist".into(),
        description: "Test cat".into(),
        license: "AGPL-3.0".into(),
        homepage: None,
        created_with: None,
        source_provenance: None,
        minimum_openpet_version: "0.1.0".into(),
        tags: vec![],
    };
    db.save_pet(&sample).unwrap();

    let pipe_name = format!(r"\\.\pipe\OpenPet-TestE2E-{}", uuid::Uuid::new_v4());
    let shutdown = Arc::new(AtomicBool::new(false));
    let handler = Arc::new(MockHostHandler { db });

    let p_clone = pipe_name.clone();
    let s_clone = shutdown.clone();
    tokio::spawn(async move {
        let _ = IpcServer::run(p_clone, handler, s_clone).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut client = IpcClient::connect(&pipe_name)
        .await
        .expect("Client should connect");

    // Ping
    let pong = client.request(&IpcRequest::Ping).await.unwrap();
    assert_eq!(pong, IpcResponse::Pong);

    // List Pets
    let pets_resp = client.request(&IpcRequest::ListPets).await.unwrap();
    if let IpcResponse::Pets(pets) = pets_resp {
        assert_eq!(pets.len(), 1);
        assert_eq!(pets[0].id.0, "mimi-cat");
    } else {
        panic!("Expected Pets response, got {:?}", pets_resp);
    }

    // Chat
    let conv_id = uuid::Uuid::new_v4();
    let chat_resp = client
        .request(&IpcRequest::SendChatMessage {
            conversation_id: conv_id,
            content: "Hello Mimi!".into(),
        })
        .await
        .unwrap();
    if let IpcResponse::ChatMessage(msg) = chat_resp {
        assert!(msg.content.contains("Hello Mimi!"));
    } else {
        panic!("Expected ChatMessage, got {:?}", chat_resp);
    }

    shutdown.store(true, Ordering::SeqCst);
}
