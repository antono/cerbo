use crate::CerboContext;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── Index JSON Structure ──────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IndexJson {
    #[serde(rename = "title_to_uuid")]
    pub title_to_uuid: std::collections::HashMap<String, String>,
    #[serde(rename = "uuid_to_path")]
    pub uuid_to_path: std::collections::HashMap<String, String>,
}

// ── Index Path ─────────────────────────────────────────────────

fn index_path(ctx: &CerboContext) -> PathBuf {
    ctx.config_dir.join("index.json")
}

// ── Load Index ────────────────────────────────────────────────

pub fn index_load(ctx: &CerboContext) -> Result<IndexJson, String> {
    let path = index_path(ctx);
    if !path.exists() {
        return Ok(IndexJson::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read index.json: {}", e))?;

    if content.trim().is_empty() {
        return Ok(IndexJson::default());
    }

    serde_json::from_str(&content).or_else(|_| {
        // Corrupted file — delete it and return empty so callers rebuild.
        let _ = fs::remove_file(&path);
        Ok(IndexJson::default())
    })
}

// ── Save Index ────────────────────────────────────────────────

pub fn index_save(ctx: &CerboContext, index: &IndexJson) -> Result<(), String> {
    let path = index_path(ctx);
    let content = serde_json::to_string_pretty(index)
        .map_err(|e| format!("Failed to serialize index.json: {}", e))?;

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write index.json: {}", e))
}

// ── Add Entry ─────────────────────────────────────────────────

pub fn index_add(ctx: &CerboContext, title: &str, uuid: &str) -> Result<(), String> {
    let mut index = index_load(ctx)?;

    index.title_to_uuid.insert(title.to_string(), uuid.to_string());

    let relative = format!("objects/{}//", uuid);
    index.uuid_to_path.insert(uuid.to_string(), relative);

    index_save(ctx, &index)
}

// ── Remove Entry ─────────────────────────────────────────────

pub fn index_remove(ctx: &CerboContext, uuid: &str) -> Result<(), String> {
    let mut index = index_load(ctx)?;

    // Remove from uuid_to_path
    index.uuid_to_path.remove(uuid);

    // Remove from title_to_uuid (find and remove by value)
    index.title_to_uuid.retain(|_, v| v != uuid);

    index_save(ctx, &index)
}

// ── Resolve Title to UUID ──────────────────────────────────

pub fn index_resolve_title(ctx: &CerboContext, title: &str) -> Result<Option<String>, String> {
    let index = index_load(ctx)?;
    Ok(index.title_to_uuid.get(title).cloned())
}

// ── Resolve UUID to Path ──────────────────────────────────

pub fn index_resolve_uuid(ctx: &CerboContext, uuid: &str) -> Result<Option<String>, String> {
    let index = index_load(ctx)?;
    Ok(index.uuid_to_path.get(uuid).cloned())
}
