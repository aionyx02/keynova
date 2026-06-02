//! Row-level INSERT / SELECT statements against the knowledge store tables.
//!
//! Extracted from `knowledge_store.rs` (REF.9.D) as a pure structural move;
//! behavior unchanged.

use rusqlite::{params, Connection};

use super::{
    ActionLogEntry, ActionStats, AgentArchiveEntry, AgentAuditEntry, AgentMemoryEntry,
    ClipboardMetadataEntry, WorkflowHistoryEntry, WorkflowHistoryRow,
};

pub(super) fn insert_action_log(conn: &Connection, entry: &ActionLogEntry) -> Result<(), String> {
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

pub(super) fn insert_action_logs(
    conn: &mut Connection,
    entries: &[ActionLogEntry],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for entry in entries {
        insert_action_log(&tx, entry)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub(super) fn insert_clipboard_metadata(
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

pub(super) fn insert_clipboard_metadata_batch(
    conn: &mut Connection,
    entries: &[ClipboardMetadataEntry],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for entry in entries {
        insert_clipboard_metadata(&tx, entry)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub(super) fn insert_agent_audit(conn: &Connection, entry: &AgentAuditEntry) -> Result<(), String> {
    conn.execute(
        "INSERT INTO agent_audit_logs (run_id, event_type, status, summary, payload_json) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            entry.run_id,
            entry.event_type,
            entry.status,
            entry.summary,
            entry.payload_json
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn insert_agent_archive(
    conn: &Connection,
    entry: &AgentArchiveEntry,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO agent_archive (run_id, prompt, status, payload_json) VALUES (?1, ?2, ?3, ?4)",
        params![entry.run_id, entry.prompt, entry.status, entry.payload_json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn insert_agent_memory(
    conn: &Connection,
    entry: &AgentMemoryEntry,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO agent_memories (id, scope, workspace_id, title, content, visibility) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET scope=excluded.scope, workspace_id=excluded.workspace_id, title=excluded.title, content=excluded.content, visibility=excluded.visibility, updated_at=strftime('%s','now')",
        params![
            entry.id,
            entry.scope,
            entry.workspace_id,
            entry.title,
            entry.content,
            entry.visibility
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn insert_workflow_history(
    conn: &Connection,
    entry: &WorkflowHistoryEntry,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO workflow_history (context_hash, route, action_label, payload_digest, workspace_id)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            entry.context_hash,
            entry.route,
            entry.action_label,
            entry.payload_digest,
            entry.workspace_id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn read_recent_workflows(
    conn: &Connection,
    context_hash: Option<&str>,
    limit: usize,
) -> Result<Vec<WorkflowHistoryRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, context_hash, route, action_label, payload_digest, workspace_id, executed_at
             FROM workflow_history
             WHERE (?1 IS NULL OR context_hash = ?1)
             ORDER BY executed_at DESC, id DESC
             LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![context_hash, limit.max(1) as i64], |row| {
            Ok(WorkflowHistoryRow {
                id: row.get(0)?,
                context_hash: row.get(1)?,
                route: row.get(2)?,
                action_label: row.get(3)?,
                payload_digest: row.get(4)?,
                workspace_id: row.get(5)?,
                executed_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub(super) fn read_action_stats(conn: &Connection, action_id: &str) -> Result<ActionStats, String> {
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

pub(super) fn read_agent_memories(
    conn: &Connection,
    scope: Option<&str>,
    workspace_id: Option<usize>,
    limit: usize,
) -> Result<Vec<AgentMemoryEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, scope, workspace_id, title, content, visibility
             FROM agent_memories
             WHERE (?1 IS NULL OR scope = ?1)
               AND (?2 IS NULL OR workspace_id = ?2)
             ORDER BY updated_at DESC
             LIMIT ?3",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![scope, workspace_id, limit.max(1) as i64,], |row| {
            Ok(AgentMemoryEntry {
                id: row.get(0)?,
                scope: row.get(1)?,
                workspace_id: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                visibility: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
