use openpet_petpack::{unpack_petpack_securely, PetPackError};
use openpet_plugin_api::{PetCommand, PluginError};
use openpet_plugin_host::PluginInstance;
use openpet_reminders::ReminderService;
use openpet_screen::ScreenPrivacyManager;
use openpet_secrets::SecretString;
use openpet_storage::Database;
use openpet_types::{BehaviorType, ToolCallProposal};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_screen_privacy_invariable_zero_capture() {
    let manager = ScreenPrivacyManager::new();
    assert!(!manager.is_analysis_enabled());

    for _ in 0..500 {
        let _ = manager.evaluate_derived_context("Confidential Document - Notepad");
    }

    // MANDATORY V1 RELEASE GATE:
    assert_eq!(
        manager.capture_initialization_count(),
        0,
        "SECURITY VIOLATION: Capture API initialized while screen analysis is disabled!"
    );
}

#[test]
fn test_zip_slip_security_boundary() {
    let temp = tempdir().unwrap();
    let malformed_zip = temp.path().join("exploit.openpet");
    let dest = temp.path().join("unpacked");

    let file = File::create(&malformed_zip).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    // Attempt path traversal payload
    zip.start_file("../../Windows/System32/calc.exe", options)
        .unwrap();
    zip.write_all(b"payload").unwrap();
    zip.finish().unwrap();

    let result = unpack_petpack_securely(&malformed_zip, &dest);
    assert!(result.is_err());
    match result {
        Err(PetPackError::PathTraversal(_)) => {}
        other => panic!("Expected PathTraversal, got {:?}", other),
    }
}

#[test]
fn test_plugin_least_privilege_and_fault_isolation() {
    let mut plugin = PluginInstance::new("third_party_mod", "Community Mod");
    // Initially has zero permissions!

    // Try executing animation without permission
    let cmd = PetCommand::PlayAnimation {
        behavior: BehaviorType::Happy,
        loops: 1,
    };
    let res = plugin.submit_command(cmd);
    assert_eq!(
        res,
        Err(PluginError::CapabilityDenied("pet.command".into()))
    );

    // Exhaust fuel
    plugin.fuel_remaining = 50;
    plugin.grant_capability(openpet_types::PluginCapability::PetCommand);
    let res_fuel = plugin.submit_command(PetCommand::PlayAnimation {
        behavior: BehaviorType::Happy,
        loops: 1,
    });
    assert_eq!(res_fuel, Err(PluginError::OutOfFuel));
    assert!(plugin.is_faulted);
    assert!(!plugin.is_enabled);
}

#[test]
fn test_tool_call_confirmation_security_gate() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let service = ReminderService::new(db);

    let proposal = ToolCallProposal {
        id: "call_abc".into(),
        name: "reminder.create".into(),
        arguments_json: r#"{"title":"Malicious Alert","body":"Test","minutes_from_now":1}"#.into(),
        requires_user_confirmation: true,
        user_confirmed: false, // NOT confirmed!
    };

    let result = service.execute_tool_proposal(&proposal);
    assert!(result.is_err());

    let mut confirmed = proposal;
    confirmed.user_confirmed = true;
    assert!(service.execute_tool_proposal(&confirmed).is_ok());
}

#[test]
fn test_secret_string_redaction_and_isolation() {
    let secret = SecretString::new("sk-proj-supersecrettoken123456789");
    let display_str = format!("{}", secret);
    let debug_str = format!("{:?}", secret);

    assert_eq!(display_str, "[REDACTED_SECRET]");
    assert_eq!(debug_str, "[REDACTED_SECRET]");
    assert!(!display_str.contains("supersecrettoken"));
    assert!(!debug_str.contains("supersecrettoken"));
}

#[test]
fn test_privacy_app_exclusion_gate() {
    let mut manager = ScreenPrivacyManager::new();
    manager.enable_screen_analysis();
    manager.add_app_exclusion("1password");
    manager.add_app_exclusion("keepass");

    // Excluded application window
    let ctx = manager.evaluate_derived_context("1Password - Vault - Personal");
    assert_eq!(ctx.activity, openpet_types::ScreenActivity::Idle);
    // Capture initialization count MUST be zero!
    assert_eq!(manager.capture_initialization_count(), 0);

    // Non-excluded window should initialize capture
    let ctx2 = manager.evaluate_derived_context("VS Code - Project.rs");
    assert_eq!(ctx2.activity, openpet_types::ScreenActivity::Writing);
    assert_eq!(manager.capture_initialization_count(), 1);
}

#[test]
fn test_manifest_id_directory_traversal_rejection() {
    use openpet_petpack::validate_pet_id;
    assert!(validate_pet_id("normal_pet-123").is_ok());
    assert!(validate_pet_id("../../system32").is_err());
    assert!(validate_pet_id("pet/name").is_err());
    assert!(validate_pet_id("con").is_err());
    assert!(validate_pet_id("aux").is_err());
}
