use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogEntry {
    pub action_id: String,
    pub action_label: String,
    pub status: String,
    pub duration_ms: u128,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardMetadataEntry {
    pub item_id: String,
    pub content_type: String,
    pub workspace_id: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStats {
    pub action_id: String,
    pub run_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbRuntimeMetrics {
    pub queue_len: usize,
    pub dropped_logs: u64,
}

pub enum DbRequest {
    WriteActionLog(ActionLogEntry),
    WriteActionLogBatch(Vec<ActionLogEntry>),
    WriteClipboardMetadata(ClipboardMetadataEntry),
    WriteClipboardMetadataBatch(Vec<ClipboardMetadataEntry>),
    ReadActionStats {
        action_id: String,
        reply: oneshot::Sender<Result<ActionStats, String>>,
    },
    Flush {
        reply: oneshot::Sender<Result<(), String>>,
    },
    Shutdown,
}

#[derive(Clone)]
pub struct KnowledgeStoreHandle {
    sender: SyncSender<DbRequest>,
    queue_len: Arc<AtomicUsize>,
    dropped_logs: Arc<AtomicU64>,
}

impl KnowledgeStoreHandle {
    pub fn new_default() -> Self {
        Self::new(default_db_path(), 1024)
    }

    pub fn new(path: PathBuf, capacity: usize) -> Self {
        let (sender, receiver) = sync_channel(capacity);
        let queue_len = Arc::new(AtomicUsize::new(0));
        let dropped_logs = Arc::new(AtomicU64::new(0));
        spawn_worker(path, receiver, Arc::clone(&queue_len));
        Self {
            sender,
            queue_len,
            dropped_logs,
        }
    }

    pub fn try_log_action(&self, entry: ActionLogEntry) {
        self.try_send_fire_and_forget(DbRequest::WriteActionLog(entry));
    }

    pub fn try_log_actions(&self, entries: Vec<ActionLogEntry>) {
        if entries.is_empty() {
            return;
        }
        self.try_send_fire_and_forget(DbRequest::WriteActionLogBatch(entries));
    }

    pub fn try_log_clipboard_metadata(&self, entry: ClipboardMetadataEntry) {
        self.try_send_fire_and_forget(DbRequest::WriteClipboardMetadata(entry));
    }

    pub fn try_log_clipboard_metadata_batch(&self, entries: Vec<ClipboardMetadataEntry>) {
        if entries.is_empty() {
            return;
        }
        self.try_send_fire_and_forget(DbRequest::WriteClipboardMetadataBatch(entries));
    }

    pub async fn action_stats(&self, action_id: String) -> Result<ActionStats, String> {
        let (reply, rx) = oneshot::channel();
        self.send_request(DbRequest::ReadActionStats { action_id, reply })?;
        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .map_err(|_| "knowledge store read timed out".to_string())?
            .map_err(|_| "knowledge store worker dropped response".to_string())?
    }

    pub async fn flush(&self) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.send_request(DbRequest::Flush { reply })?;
        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .map_err(|_| "knowledge store flush timed out".to_string())?
            .map_err(|_| "knowledge store worker dropped response".to_string())?
    }

    pub fn metrics(&self) -> DbRuntimeMetrics {
        DbRuntimeMetrics {
            queue_len: self.queue_len.load(Ordering::Relaxed),
            dropped_logs: self.dropped_logs.load(Ordering::Relaxed),
        }
    }

    fn send_request(&self, request: DbRequest) -> Result<(), String> {
        self.queue_len.fetch_add(1, Ordering::Relaxed);
        match self.sender.try_send(request) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.queue_len.fetch_sub(1, Ordering::Relaxed);
                Err("knowledge store queue is full".to_string())
            }
            Err(TrySendError::Disconnected(_)) => {
                self.queue_len.fetch_sub(1, Ordering::Relaxed);
                Err("knowledge store worker is stopped".to_string())
            }
        }
    }

    fn try_send_fire_and_forget(&self, request: DbRequest) {
        self.queue_len.fetch_add(1, Ordering::Relaxed);
        match self.sender.try_send(request) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.queue_len.fetch_sub(1, Ordering::Relaxed);
                self.dropped_logs.fetch_add(1, Ordering::Relaxed);
            }
            Err(TrySendError::Disconnected(_)) => {
                self.queue_len.fetch_sub(1, Ordering::Relaxed);
                self.dropped_logs.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

fn spawn_worker(path: PathBuf, receiver: Receiver<DbRequest>, queue_len: Arc<AtomicUsize>) {
    std::thread::spawn(move || {
        let mut conn = match open_connection(&path) {
            Ok(conn) => conn,
            Err(error) => {
                eprintln!("[keynova][db] worker disabled: {error}");
                drain_failed(receiver, queue_len, error);
                return;
            }
        };

        while let Ok(request) = receiver.recv() {
            queue_len.fetch_sub(1, Ordering::Relaxed);
            let started = Instant::now();
            let result = handle_request(&mut conn, request);
            crate::core::observability::log_db_request(
                "knowledge_store",
                result.is_ok(),
                started.elapsed(),
            );
            if matches!(result, Ok(WorkerSignal::Shutdown)) {
                break;
            }
        }
    });
}

fn drain_failed(receiver: Receiver<DbRequest>, queue_len: Arc<AtomicUsize>, error: String) {
    for request in receiver {
        queue_len.fetch_sub(1, Ordering::Relaxed);
        respond_error(request, error.clone());
    }
}

enum WorkerSignal {
    Continue,
    Shutdown,
}

fn handle_request(conn: &mut Connection, request: DbRequest) -> Result<WorkerSignal, String> {
    match request {
        DbRequest::WriteActionLog(entry) => {
            insert_action_log(conn, &entry)?;
            Ok(WorkerSignal::Continue)
        }
        DbRequest::WriteActionLogBatch(entries) => {
            insert_action_logs(conn, &entries)?;
            Ok(WorkerSignal::Continue)
        }
        DbRequest::WriteClipboardMetadata(entry) => {
            insert_clipboard_metadata(conn, &entry)?;
            Ok(WorkerSignal::Continue)
        }
        DbRequest::WriteClipboardMetadataBatch(entries) => {
            insert_clipboard_metadata_batch(conn, &entries)?;
            Ok(WorkerSignal::Continue)
        }
        DbRequest::ReadActionStats { action_id, reply } => {
            let result = read_action_stats(conn, &action_id);
            let _ = reply.send(result);
            Ok(WorkerSignal::Continue)
        }
        DbRequest::Flush { reply } => {
            let result = conn
                .execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
                .map_err(|e| e.to_string());
            let _ = reply.send(result);
            Ok(WorkerSignal::Continue)
        }
        DbRequest::Shutdown => Ok(WorkerSignal::Shutdown),
    }
}

fn respond_error(request: DbRequest, error: String) {
    match request {
        DbRequest::ReadActionStats { reply, .. } => {
            let _ = reply.send(Err(error));
        }
        DbRequest::Flush { reply } => {
            let _ = reply.send(Err(error));
        }
        DbRequest::WriteActionLog(_)
        | DbRequest::WriteActionLogBatch(_)
        | DbRequest::WriteClipboardMetadata(_)
        | DbRequest::WriteClipboardMetadataBatch(_)
        | DbRequest::Shutdown => {}
    }
}

fn open_connection(path: &Path) -> Result<Connection, String> {
    let existed_before_open = path.exists();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
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

fn read_user_version(conn: &Connection) -> Result<u32, String> {
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

fn insert_action_log(conn: &Connection, entry: &ActionLogEntry) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO actions (id, label) VALUES (?1, ?2)",
        params![entry.action_id, entry.action_label],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO action_logs (action_id, action_label, status, duration_ms, error) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            entry.action_id,
            entry.action_label,
            entry.status,
            entry.duration_ms.to_string(),
            entry.error
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn insert_action_logs(conn: &mut Connection, entries: &[ActionLogEntry]) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for entry in entries {
        insert_action_log(&tx, entry)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

fn insert_clipboard_metadata(
    conn: &Connection,
    entry: &ClipboardMetadataEntry,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO clipboard_items (item_id, content_type, workspace_id) VALUES (?1, ?2, ?3)
         ON CONFLICT(item_id) DO UPDATE SET content_type=excluded.content_type, workspace_id=excluded.workspace_id, updated_at=strftime('%s','now')",
        params![entry.item_id, entry.content_type, entry.workspace_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn insert_clipboard_metadata_batch(
    conn: &mut Connection,
    entries: &[ClipboardMetadataEntry],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for entry in entries {
        insert_clipboard_metadata(&tx, entry)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

fn read_action_stats(conn: &Connection, action_id: &str) -> Result<ActionStats, String> {
    let run_count = conn
        .query_row(
            "SELECT COUNT(*) FROM action_logs WHERE action_id = ?1",
            params![action_id],
            |row| row.get::<_, u64>(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(ActionStats {
        action_id: action_id.to_string(),
        run_count,
    })
}

fn default_db_path() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("Keynova").join("knowledge.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("keynova-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("knowledge.db")
    }

    fn cleanup_db_path(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[tokio::test]
    async fn writes_action_logs_on_worker_thread() {
        let path = test_db_path("action-log");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            store.try_log_action(ActionLogEntry {
                action_id: "cmd:help".into(),
                action_label: "Help".into(),
                status: "ok".into(),
                duration_ms: 3,
                error: None,
            });
            store.flush().await.unwrap();
            let stats = store.action_stats("cmd:help".into()).await.unwrap();
            assert_eq!(stats.run_count, 1);
        }
        cleanup_db_path(&path);
    }

    #[tokio::test]
    async fn batch_writes_action_logs_on_worker_thread() {
        let path = test_db_path("action-log-batch");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            store.try_log_actions(vec![
                ActionLogEntry {
                    action_id: "cmd:help".into(),
                    action_label: "Help".into(),
                    status: "ok".into(),
                    duration_ms: 3,
                    error: None,
                },
                ActionLogEntry {
                    action_id: "cmd:help".into(),
                    action_label: "Help".into(),
                    status: "ok".into(),
                    duration_ms: 5,
                    error: None,
                },
            ]);
            store.flush().await.unwrap();
            let stats = store.action_stats("cmd:help".into()).await.unwrap();
            assert_eq!(stats.run_count, 2);
        }
        cleanup_db_path(&path);
    }

    #[test]
    fn new_connection_sets_current_schema_version() {
        let path = test_db_path("new-schema");
        let conn = open_connection(&path).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
        drop(conn);
        cleanup_db_path(&path);
    }

    #[test]
    fn existing_database_is_backed_up_before_migration() {
        let path = test_db_path("migration-backup");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE legacy_data (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                INSERT INTO legacy_data (name) VALUES ('before');
                PRAGMA user_version = 1;
                "#,
            )
            .unwrap();
        }

        let conn = open_connection(&path).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
        let backup_path: String = conn
            .query_row(
                "SELECT backup_path FROM schema_migrations WHERE version = ?1",
                params![CURRENT_SCHEMA_VERSION],
                |row| row.get(0),
            )
            .unwrap();
        assert!(PathBuf::from(backup_path).exists());
        drop(conn);
        cleanup_db_path(&path);
    }
}
