//! Background DB worker thread: owns the `Connection`, drains the request
//! channel, and dispatches each `DbRequest` to the row-level statements.
//!
//! Extracted from `knowledge_store.rs` (REF.9.D) as a pure structural move;
//! behavior unchanged.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

use rusqlite::Connection;

use super::schema::open_connection;
use super::sql::{
    insert_action_log, insert_action_logs, insert_agent_archive, insert_agent_audit,
    insert_agent_memory, insert_clipboard_metadata, insert_clipboard_metadata_batch,
    insert_workflow_history, read_action_stats, read_agent_memories, read_recent_workflows,
};
use super::DbRequest;

pub(super) fn spawn_worker(
    path: PathBuf,
    receiver: Receiver<DbRequest>,
    queue_len: Arc<AtomicUsize>,
) {
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
        DbRequest::WriteAgentArchive(entry) => {
            insert_agent_archive(conn, &entry)?;
            Ok(WorkerSignal::Continue)
        }
        DbRequest::WriteAgentAudit(entry) => {
            insert_agent_audit(conn, &entry)?;
            Ok(WorkerSignal::Continue)
        }
        DbRequest::WriteAgentMemory(entry) => {
            insert_agent_memory(conn, &entry)?;
            Ok(WorkerSignal::Continue)
        }
        DbRequest::WriteWorkflowHistory(entry) => {
            insert_workflow_history(conn, &entry)?;
            Ok(WorkerSignal::Continue)
        }
        DbRequest::ReadActionStats { action_id, reply } => {
            let result = read_action_stats(conn, &action_id);
            let _ = reply.send(result);
            Ok(WorkerSignal::Continue)
        }
        DbRequest::ReadAgentMemories {
            scope,
            workspace_id,
            limit,
            reply,
        } => {
            let result = read_agent_memories(conn, scope.as_deref(), workspace_id, limit);
            let _ = reply.send(result);
            Ok(WorkerSignal::Continue)
        }
        DbRequest::ReadRecentWorkflows {
            context_hash,
            limit,
            reply,
        } => {
            let result = read_recent_workflows(conn, context_hash.as_deref(), limit);
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
        DbRequest::ReadAgentMemories { reply, .. } => {
            let _ = reply.send(Err(error));
        }
        DbRequest::ReadRecentWorkflows { reply, .. } => {
            let _ = reply.send(Err(error));
        }
        DbRequest::Flush { reply } => {
            let _ = reply.send(Err(error));
        }
        DbRequest::WriteActionLog(_)
        | DbRequest::WriteActionLogBatch(_)
        | DbRequest::WriteClipboardMetadata(_)
        | DbRequest::WriteClipboardMetadataBatch(_)
        | DbRequest::WriteAgentAudit(_)
        | DbRequest::WriteAgentArchive(_)
        | DbRequest::WriteAgentMemory(_)
        | DbRequest::WriteWorkflowHistory(_)
        | DbRequest::Shutdown => {}
    }
}
