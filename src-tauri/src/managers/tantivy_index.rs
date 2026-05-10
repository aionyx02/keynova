use std::path::{Path, PathBuf};

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, TantivyDocument, Value, STORED, STRING, TEXT};
use tantivy::{doc, Index};

use crate::models::search_result::{ResultKind, SearchResult};

#[derive(Debug, Clone)]
pub struct TantivyFileEntry {
    pub name: String,
    pub path: String,
    pub is_folder: bool,
}

#[derive(Debug, Clone)]
struct TantivyFields {
    name: Field,
    path: Field,
    kind: Field,
    is_folder: Field,
}

pub fn default_index_dir() -> PathBuf {
    crate::platform_dirs::keynova_data_dir()
        .join("search")
        .join("tantivy")
}

pub fn resolve_index_dir(configured: Option<&str>) -> PathBuf {
    configured
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_index_dir)
}

pub fn indexed_entries(index_dir: &Path) -> usize {
    let Ok(index) = Index::open_in_dir(index_dir) else {
        return 0;
    };
    let Ok(reader) = index.reader() else {
        return 0;
    };
    reader.searcher().num_docs() as usize
}

pub fn rebuild(index_dir: &Path, entries: &[TantivyFileEntry]) -> Result<usize, String> {
    if let Some(parent) = index_dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if index_dir.exists() {
        let _ = std::fs::remove_dir_all(index_dir);
    }
    std::fs::create_dir_all(index_dir).map_err(|e| e.to_string())?;

    let (schema, fields) = build_schema();
    let index = Index::create_in_dir(index_dir, schema).map_err(|e| e.to_string())?;
    // 15 MB is sufficient for indexing; 50 MB was wasting address space during rebuild.
    let mut writer = index.writer(30_000_000).map_err(|e| e.to_string())?;
    for entry in entries {
        if entry.name.contains('\u{FFFD}') || entry.path.contains('\u{FFFD}') {
            eprintln!("[keynova] tantivy: skipping entry with non-UTF-8 path: {:?}", entry.path);
            continue;
        }
        let kind = if entry.is_folder { "folder" } else { "file" };
        writer
            .add_document(doc!(
                fields.name => entry.name.as_str(),
                fields.path => entry.path.as_str(),
                fields.kind => kind,
                fields.is_folder => if entry.is_folder { "true" } else { "false" },
            ))
            .map_err(|e| e.to_string())?;
    }
    writer.commit().map_err(|e| e.to_string())?;
    drop(writer); // release the 15 MB index buffer immediately
    Ok(entries.len())
}

pub fn search(index_dir: &Path, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let index = Index::open_in_dir(index_dir).map_err(|e| e.to_string())?;
    let schema = index.schema();
    let fields = fields_from_schema(&schema)?;
    let reader = index.reader().map_err(|e| e.to_string())?;
    let searcher = reader.searcher();
    let parser = QueryParser::for_index(&index, vec![fields.name, fields.path]);
    let parsed = parser
        .parse_query(query)
        .or_else(|_| parser.parse_query(&format!("name:{query}*")))
        .map_err(|e| e.to_string())?;
    let docs = searcher
        .search(&parsed, &TopDocs::with_limit(limit).order_by_score())
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for (score, address) in docs {
        let doc: TantivyDocument = searcher.doc(address).map_err(|e| e.to_string())?;
        let name = first_text(&doc, fields.name).unwrap_or_default();
        let path = first_text(&doc, fields.path).unwrap_or_default();
        let is_folder = first_text(&doc, fields.is_folder).as_deref() == Some("true");
        if name.is_empty() || path.is_empty() {
            continue;
        }
        results.push(SearchResult {
            kind: if is_folder {
                ResultKind::Folder
            } else {
                ResultKind::File
            },
            name,
            path,
            score: 80 + score.round() as i64,
        });
    }
    Ok(results)
}

fn build_schema() -> (Schema, TantivyFields) {
    let mut builder = Schema::builder();
    let name = builder.add_text_field("name", TEXT | STORED);
    let path = builder.add_text_field("path", TEXT | STORED);
    let kind = builder.add_text_field("kind", STRING | STORED);
    let is_folder = builder.add_text_field("is_folder", STRING | STORED);
    let schema = builder.build();
    (
        schema,
        TantivyFields {
            name,
            path,
            kind,
            is_folder,
        },
    )
}

fn fields_from_schema(schema: &Schema) -> Result<TantivyFields, String> {
    Ok(TantivyFields {
        name: schema
            .get_field("name")
            .map_err(|_| "tantivy index missing name field".to_string())?,
        path: schema
            .get_field("path")
            .map_err(|_| "tantivy index missing path field".to_string())?,
        kind: schema
            .get_field("kind")
            .map_err(|_| "tantivy index missing kind field".to_string())?,
        is_folder: schema
            .get_field("is_folder")
            .map_err(|_| "tantivy index missing is_folder field".to_string())?,
    })
}

fn first_text(doc: &TantivyDocument, field: Field) -> Option<String> {
    doc.get_first(field)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuild_and_search_persisted_index() {
        let dir = std::env::temp_dir().join(format!("keynova-tantivy-{}", uuid::Uuid::new_v4()));
        let entries = vec![TantivyFileEntry {
            name: "project_notes.md".into(),
            path: "C:/tmp/project_notes.md".into(),
            is_folder: false,
        }];
        assert_eq!(rebuild(&dir, &entries).unwrap(), 1);
        assert_eq!(indexed_entries(&dir), 1);
        let results = search(&dir, "project", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "project_notes.md");
        let _ = std::fs::remove_dir_all(dir);
    }
}
