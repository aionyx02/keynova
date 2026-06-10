//! SQLite connection setup, schema initialization, versioning, and
//! pre-migration backup.
//!
//! Focused schema module used by the knowledge store facade.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::models::settings_schema::builtin_setting_schema;

pub(super) const CURRENT_SCHEMA_VERSION: u32 = 7;

pub(super) fn open_connection(path: &Path) -> Result<Connection, String> {
    let existed_before_open = path.exists();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.pragma_update(None, "busy_timeout", 2500)
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;

    let previous_version = read_user_version(&conn)?;
    let backup_path = if existed_before_open && previous_version < CURRENT_SCHEMA_VERSION {
        backup_before_migration(&conn, path, previous_version, CURRENT_SCHEMA_VERSION)?
    } else {
        None
    };

    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| e.to_string())?;
    init_schema(&conn)?;
    // ADR-0053: additive `succeeded` column. `CREATE TABLE IF NOT EXISTS` above
    // covers fresh DBs; existing DBs need an idempotent ALTER.
    ensure_workflow_succeeded_column(&conn)?;
    if previous_version < CURRENT_SCHEMA_VERSION {
        sanitize_sensitive_workflow_history_labels(&mut conn)?;
    }
    if previous_version < CURRENT_SCHEMA_VERSION {
        record_schema_migration(
            &conn,
            previous_version,
            CURRENT_SCHEMA_VERSION,
            backup_path.as_deref(),
        )?;
        set_user_version(&conn, CURRENT_SCHEMA_VERSION)?;
    }
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS actions (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS action_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action_id TEXT NOT NULL,
            action_label TEXT NOT NULL,
            status TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            error TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS clipboard_items (
            item_id TEXT PRIMARY KEY,
            content_type TEXT NOT NULL,
            workspace_id INTEGER,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS notes_index (
            note_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            workspace_id INTEGER,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS ai_conversations (
            id TEXT PRIMARY KEY,
            workspace_id INTEGER,
            title TEXT,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS workspace_contexts (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            project_root TEXT,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS search_index_metadata (
            provider TEXT PRIMARY KEY,
            generation INTEGER NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS workflow_definitions (
            name TEXT PRIMARY KEY,
            definition TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS agent_audit_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            status TEXT NOT NULL,
            summary TEXT NOT NULL,
            payload_json TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS agent_memories (
            id TEXT PRIMARY KEY,
            scope TEXT NOT NULL,
            workspace_id INTEGER,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            visibility TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS workflow_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            context_hash TEXT,
            route TEXT NOT NULL,
            action_label TEXT NOT NULL,
            payload_digest TEXT,
            workspace_id INTEGER,
            succeeded INTEGER,
            project_root TEXT,
            executed_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE INDEX IF NOT EXISTS idx_workflow_history_context
            ON workflow_history(context_hash, executed_at DESC);
        CREATE INDEX IF NOT EXISTS idx_workflow_history_executed
            ON workflow_history(executed_at DESC);
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            previous_version INTEGER NOT NULL,
            backup_path TEXT,
            applied_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        "#,
    )
    .map_err(|e| e.to_string())
}

/// Add the nullable `workflow_history` columns introduced after the table's
/// original schema (`succeeded` ADR-0053, `project_root` ADR-0054) on upgrade.
/// Idempotent — checks `PRAGMA table_info` per column, so re-runs and fresh DBs
/// (which already have the columns from `CREATE TABLE`) are no-ops.
fn ensure_workflow_succeeded_column(conn: &Connection) -> Result<(), String> {
    ensure_workflow_column(conn, "succeeded", "INTEGER")?;
    ensure_workflow_column(conn, "project_root", "TEXT")?;
    Ok(())
}

fn ensure_workflow_column(conn: &Connection, column: &str, ty: &str) -> Result<(), String> {
    let has_column = conn
        .prepare("PRAGMA table_info(workflow_history)")
        .and_then(|mut stmt| {
            let names = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(names.iter().any(|name| name == column))
        })
        .map_err(|e| e.to_string())?;
    if !has_column {
        conn.execute_batch(&format!(
            "ALTER TABLE workflow_history ADD COLUMN {column} {ty};"
        ))
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn read_user_version(conn: &Connection) -> Result<u32, String> {
    conn.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
        .map_err(|e| e.to_string())
}

fn set_user_version(conn: &Connection, version: u32) -> Result<(), String> {
    conn.execute_batch(&format!("PRAGMA user_version = {version};"))
        .map_err(|e| e.to_string())
}

fn record_schema_migration(
    conn: &Connection,
    previous_version: u32,
    version: u32,
    backup_path: Option<&Path>,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO schema_migrations (version, previous_version, backup_path) VALUES (?1, ?2, ?3)",
        params![
            version,
            previous_version,
            backup_path.map(|path| path.display().to_string())
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn backup_before_migration(
    conn: &Connection,
    path: &Path,
    previous_version: u32,
    version: u32,
) -> Result<Option<PathBuf>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let backup_path = migration_backup_path(path, previous_version, version);
    if let Some(parent) = backup_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(|e| e.to_string())?;
    std::fs::copy(path, &backup_path).map_err(|e| e.to_string())?;
    copy_sidecar_if_present(path, &backup_path, "-wal")?;
    copy_sidecar_if_present(path, &backup_path, "-shm")?;
    Ok(Some(backup_path))
}

fn migration_backup_path(path: &Path, previous_version: u32, version: u32) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("knowledge");
    let id = uuid::Uuid::new_v4();
    parent
        .join("backups")
        .join(format!("{stem}-v{previous_version}-to-v{version}-{id}.db"))
}

fn sanitize_sensitive_workflow_history_labels(conn: &mut Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for key in builtin_setting_schema()
        .into_iter()
        .filter(|schema| schema.sensitive)
        .map(|schema| schema.key)
    {
        tx.execute(
            "UPDATE workflow_history
             SET action_label = ?1
             WHERE action_label LIKE ?2",
            params![
                format!("/setting {key} [redacted]"),
                format!("/setting {key} %")
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

fn copy_sidecar_if_present(source_db: &Path, backup_db: &Path, suffix: &str) -> Result<(), String> {
    let Some(source_name) = source_db.file_name().and_then(|value| value.to_str()) else {
        return Ok(());
    };
    let Some(backup_name) = backup_db.file_name().and_then(|value| value.to_str()) else {
        return Ok(());
    };
    let source = source_db.with_file_name(format!("{source_name}{suffix}"));
    if !source.exists() {
        return Ok(());
    }
    let backup = backup_db.with_file_name(format!("{backup_name}{suffix}"));
    std::fs::copy(source, backup).map_err(|e| e.to_string())?;
    Ok(())
}
