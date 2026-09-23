use rusqlite::{Connection, Result};
use tracing::info;

pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial_openpet_schema",
    sql: r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS pets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                author TEXT NOT NULL,
                description TEXT NOT NULL,
                license TEXT NOT NULL,
                homepage TEXT,
                created_with TEXT,
                provenance TEXT,
                min_version TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS pet_installations (
                pet_id TEXT PRIMARY KEY,
                installed_at TEXT NOT NULL,
                pack_dir TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tool_call_id TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS memory_facts (
                id TEXT PRIMARY KEY,
                subject TEXT NOT NULL,
                predicate TEXT NOT NULL,
                object TEXT NOT NULL,
                source_message_id TEXT,
                confidence REAL NOT NULL,
                user_locked INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS reminders (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                schedule TEXT NOT NULL,
                recurrence TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                last_fired_at TEXT
            );

            CREATE TABLE IF NOT EXISTS reminder_runs (
                id TEXT PRIMARY KEY,
                reminder_id TEXT NOT NULL,
                fired_at TEXT NOT NULL,
                acknowledged_at TEXT,
                FOREIGN KEY (reminder_id) REFERENCES reminders(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS provider_profiles (
                kind TEXT PRIMARY KEY,
                base_url TEXT,
                model_name TEXT NOT NULL,
                is_enabled INTEGER NOT NULL DEFAULT 0,
                timeout_seconds INTEGER NOT NULL DEFAULT 60
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS privacy_exclusions (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS plugins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                description TEXT NOT NULL,
                author TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS plugin_permissions (
                plugin_id TEXT NOT NULL,
                permission TEXT NOT NULL,
                granted INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (plugin_id, permission)
            );

            CREATE TABLE IF NOT EXISTS generation_jobs (
                id TEXT PRIMARY KEY,
                pet_name TEXT NOT NULL,
                state TEXT NOT NULL,
                progress REAL NOT NULL,
                photo_count INTEGER NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
        "#,
}];

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;

    for migration in MIGRATIONS {
        let applied: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [migration.version],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !applied {
            info!(
                "Applying database migration v{}: {}",
                migration.version, migration.name
            );
            let tx = conn.transaction()?;
            tx.execute_batch(migration.sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
                (migration.version, migration.name),
            )?;
            tx.commit()?;
        }
    }

    Ok(())
}
