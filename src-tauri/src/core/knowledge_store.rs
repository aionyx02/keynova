use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

mod schema;
mod sql;
mod worker;

use worker::spawn_worker;

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
pub struct AgentAuditEntry {
    pub run_id: String,
    pub event_type: String,
    pub status: String,
    pub summary: String,
    pub payload_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentArchiveEntry {
    pub run_id: String,
    pub prompt: String,
    pub status: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryEntry {
    pub id: String,
    pub scope: String,
    pub workspace_id: Option<usize>,
    pub title: String,
    pub content: String,
    pub visibility: String,
}

/// REF.5 — write side of `workflow_history` (schema v4). Recorded fire-and-
/// forget at the `cmd_dispatch_impl` chokepoint for allowlisted routes
/// (`action.run`, `cmd.run`, `capability.call`). `id` and `executed_at` are
/// server-assigned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowHistoryEntry {
    pub context_hash: Option<String>,
    pub route: String,
    pub action_label: String,
    pub payload_digest: Option<String>,
    pub workspace_id: Option<i64>,
}

/// REF.5 — read side of `workflow_history`. Carries the server-assigned
/// `id` + `executed_at` so callers can render and key UI rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowHistoryRow {
    pub id: i64,
    pub context_hash: Option<String>,
    pub route: String,
    pub action_label: String,
    pub payload_digest: Option<String>,
    pub workspace_id: Option<i64>,
    pub executed_at: i64,
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
    WriteAgentAudit(AgentAuditEntry),
    WriteAgentArchive(AgentArchiveEntry),
    WriteAgentMemory(AgentMemoryEntry),
    WriteWorkflowHistory(WorkflowHistoryEntry),
    ReadActionStats {
        action_id: String,
        reply: oneshot::Sender<Result<ActionStats, String>>,
    },
    ReadAgentMemories {
        scope: Option<String>,
        workspace_id: Option<usize>,
        limit: usize,
        reply: oneshot::Sender<Result<Vec<AgentMemoryEntry>, String>>,
    },
    ReadRecentWorkflows {
        context_hash: Option<String>,
        limit: usize,
        reply: oneshot::Sender<Result<Vec<WorkflowHistoryRow>, String>>,
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

    pub fn try_log_agent_audit(&self, entry: AgentAuditEntry) {
        self.try_send_fire_and_forget(DbRequest::WriteAgentAudit(entry));
    }

    pub fn try_archive_agent_run(&self, entry: AgentArchiveEntry) {
        self.try_send_fire_and_forget(DbRequest::WriteAgentArchive(entry));
    }

    pub fn try_store_agent_memory(&self, entry: AgentMemoryEntry) {
        self.try_send_fire_and_forget(DbRequest::WriteAgentMemory(entry));
    }

    /// REF.5 — fire-and-forget record into `workflow_history`. Mirrors the
    /// agent-audit pattern; failures bump `dropped_logs` but never block
    /// the caller (workflow recording is best-effort).
    pub fn try_log_workflow_history(&self, entry: WorkflowHistoryEntry) {
        self.try_send_fire_and_forget(DbRequest::WriteWorkflowHistory(entry));
    }

    /// REF.5 — async read of recent workflow history. When
    /// `context_hash` is `Some(...)`, results are filtered to rows with
    /// matching hash; otherwise returns the global top-N by `executed_at`.
    pub async fn recent_workflows(
        &self,
        context_hash: Option<String>,
        limit: usize,
    ) -> Result<Vec<WorkflowHistoryRow>, String> {
        let (reply, rx) = oneshot::channel();
        self.send_request(DbRequest::ReadRecentWorkflows {
            context_hash,
            limit,
            reply,
        })?;
        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .map_err(|_| "knowledge store read timed out".to_string())?
            .map_err(|_| "knowledge store worker dropped response".to_string())?
    }

    /// REF.5 — blocking read for synchronous handler call sites. Same
    /// semantics as `recent_workflows`.
    pub fn recent_workflows_blocking(
        &self,
        context_hash: Option<String>,
        limit: usize,
    ) -> Result<Vec<WorkflowHistoryRow>, String> {
        let (reply, rx) = oneshot::channel();
        self.send_request(DbRequest::ReadRecentWorkflows {
            context_hash,
            limit,
            reply,
        })?;
        rx.blocking_recv()
            .map_err(|_| "knowledge store worker dropped response".to_string())?
    }

    pub async fn action_stats(&self, action_id: String) -> Result<ActionStats, String> {
        let (reply, rx) = oneshot::channel();
        self.send_request(DbRequest::ReadActionStats { action_id, reply })?;
        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .map_err(|_| "knowledge store read timed out".to_string())?
            .map_err(|_| "knowledge store worker dropped response".to_string())?
    }

    pub async fn agent_memories(
        &self,
        scope: Option<String>,
        workspace_id: Option<usize>,
        limit: usize,
    ) -> Result<Vec<AgentMemoryEntry>, String> {
        let (reply, rx) = oneshot::channel();
        self.send_request(DbRequest::ReadAgentMemories {
            scope,
            workspace_id,
            limit,
            reply,
        })?;
        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .map_err(|_| "knowledge store read timed out".to_string())?
            .map_err(|_| "knowledge store worker dropped response".to_string())?
    }

    pub fn agent_memories_blocking(
        &self,
        scope: Option<String>,
        workspace_id: Option<usize>,
        limit: usize,
    ) -> Result<Vec<AgentMemoryEntry>, String> {
        let (reply, rx) = oneshot::channel();
        self.send_request(DbRequest::ReadAgentMemories {
            scope,
            workspace_id,
            limit,
            reply,
        })?;
        rx.blocking_recv()
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

fn default_db_path() -> PathBuf {
    crate::platform_dirs::keynova_data_dir().join("knowledge.db")
}

#[cfg(test)]
mod tests {
    use super::schema::{open_connection, read_user_version, CURRENT_SCHEMA_VERSION};
    use super::*;
    use rusqlite::{params, Connection};
    use std::path::Path;

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
    fn stores_and_reads_agent_memories() {
        let path = test_db_path("agent-memory");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            store.try_store_agent_memory(AgentMemoryEntry {
                id: "run:1".into(),
                scope: "long_term".into(),
                workspace_id: Some(1),
                title: "Recent plan".into(),
                content: "Approved note draft".into(),
                visibility: "user_private".into(),
            });
            let _ = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(store.flush())
                .unwrap();
            let memories = store
                .agent_memories_blocking(Some("long_term".into()), Some(1), 5)
                .unwrap();
            assert_eq!(memories.len(), 1);
            assert_eq!(memories[0].title, "Recent plan");
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

    #[test]
    fn migration_redacts_sensitive_setting_workflow_labels() {
        let path = test_db_path("sanitize-workflow-history");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE workflow_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    context_hash TEXT,
                    route TEXT NOT NULL,
                    action_label TEXT NOT NULL,
                    payload_digest TEXT,
                    workspace_id INTEGER,
                    executed_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                );
                INSERT INTO workflow_history (route, action_label)
                VALUES ('cmd.run', '/setting translation.api_key secret-value');
                PRAGMA user_version = 4;
                "#,
            )
            .unwrap();
        }

        let conn = open_connection(&path).unwrap();
        let label: String = conn
            .query_row(
                "SELECT action_label FROM workflow_history LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(label, "/setting translation.api_key [redacted]");
        assert_eq!(read_user_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
        drop(conn);
        cleanup_db_path(&path);
    }
}
