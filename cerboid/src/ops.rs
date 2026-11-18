//! The vault operations behind the RPC calls. Every function here blocks and
//! is meant for a blocking thread: reads straight from `spawn_blocking`,
//! writes from the vault's write queue.

use crate::embed::EmbedWorker;
use crate::registry::VaultHandle;
use cerbo_core::object::{self, ObjectMeta, ObjectType};
use cerbo_core::rpc::RpcError;
use cerbo_core::semantic::{self, IndexOptions, SearchOptions, SemanticError};
use serde_json::{json, Value};

pub type OpResult = Result<Value, RpcError>;

fn failed(e: impl std::fmt::Display) -> RpcError {
    RpcError::failed(e.to_string())
}

fn sem_err(e: SemanticError) -> RpcError {
    RpcError::failed(e.to_string())
}

/// A page id from a client, in its one canonical spelling. Parsing it as a
/// UUID before it is used as a path segment keeps a hostile id away from the
/// filesystem.
pub fn page_id(id: &str) -> Result<String, RpcError> {
    uuid::Uuid::parse_str(id)
        .map(|u| u.hyphenated().to_string())
        .map_err(|_| RpcError::failed(format!("not a page id: {id:?}")))
}

fn existing(vault: &VaultHandle, id: &str) -> Result<String, RpcError> {
    let id = page_id(id)?;
    if !object::object_path(&vault.page_ctx, &id).is_dir() {
        return Err(RpcError::failed(format!("Object not found: {id}")));
    }
    Ok(id)
}

fn meta_of(vault: &VaultHandle, id: &str) -> Result<ObjectMeta, RpcError> {
    let path = object::object_path(&vault.page_ctx, id).join("meta.ttl");
    ObjectMeta::read_from_file(&path).map_err(|e| failed(format!("cannot read the metadata of {id}: {e}")))
}

// ── Reads ────────────────────────────────────────────────────────────────────

pub fn page_list(vault: &VaultHandle) -> OpResult {
    let pages = cerbo_core::page::page_list(&vault.page_ctx).map_err(failed)?;
    serde_json::to_value(pages).map_err(failed)
}

pub fn page_read(vault: &VaultHandle, id: &str) -> OpResult {
    let id = existing(vault, id)?;
    let content = object::object_read(&vault.page_ctx, &id).map_err(failed)?;
    let meta = meta_of(vault, &id)?;
    Ok(json!({
        "uuid": id,
        "title": meta.title,
        "version": version_text(fingerprint(&content)),
        "content": content,
        "type": meta.object_type.as_str(),
        "virtualPath": meta.virtual_path,
        "modified": meta.modified,
    }))
}

pub fn attachment_list(vault: &VaultHandle, page: Option<&str>) -> OpResult {
    if let Some(page) = page {
        let id = existing(vault, page)?;
        let infos = object::attachment_infos(&vault.page_ctx, &id).map_err(failed)?;
        return serde_json::to_value(infos).map_err(failed);
    }
    let objects = cerbo_core::vault::list_vault_objects(&vault.page_ctx, &vault.root).map_err(failed)?;
    let mut attachments: Vec<Value> = Vec::new();
    for o in objects.into_iter().filter(|o| o.object_type == ObjectType::Attachment.as_str()) {
        let mime = meta_of(vault, &o.uuid).ok().and_then(|m| m.mime_type);
        attachments.push(json!({ "uuid": o.uuid, "title": o.title, "mime_type": mime }));
    }
    attachments.sort_by(|a, b| a["title"].as_str().cmp(&b["title"].as_str()));
    Ok(Value::Array(attachments))
}

pub fn attachment_info(vault: &VaultHandle, id: &str) -> OpResult {
    let id = page_id(id)?;
    let path = object::attachment_file_path(&vault.page_ctx, &id).map_err(failed)?;
    let meta = meta_of(vault, &id)?;
    Ok(json!({
        "uuid": id,
        "title": meta.title,
        "mime_type": meta.mime_type,
        "path": path.to_string_lossy(),
    }))
}

/// The live `cerbo://` targets of `content`, each once, in order.
fn live_links(vault: &VaultHandle, content: &str) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for target in cerbo_core::links::extract_cerbo_links(content) {
        if let Ok(id) = page_id(&target)
            && cerbo_core::links::is_live_target(&vault.page_ctx, &id)
            && !seen.contains(&id)
        {
            seen.push(id);
        }
    }
    seen
}

pub fn links(vault: &VaultHandle) -> OpResult {
    let mut pairs = Vec::new();
    for page in cerbo_core::page::page_list(&vault.page_ctx).map_err(failed)? {
        let Ok(content) = object::object_read(&vault.page_ctx, &page.uuid) else { continue };
        for to in live_links(vault, &content) {
            pairs.push(json!({ "from": page.uuid, "to": to }));
        }
    }
    Ok(Value::Array(pairs))
}

pub fn outgoing_links(vault: &VaultHandle, id: &str) -> OpResult {
    let id = existing(vault, id)?;
    let content = object::object_read(&vault.page_ctx, &id).map_err(failed)?;
    let targets: Vec<Value> = live_links(vault, &content)
        .into_iter()
        .map(|to| {
            let title = meta_of(vault, &to).map(|m| m.title).unwrap_or_default();
            json!({ "uuid": to, "title": title })
        })
        .collect();
    Ok(Value::Array(targets))
}

pub fn backlinks(vault: &VaultHandle, id: &str) -> OpResult {
    let id = page_id(id)?;
    let _gate = vault.index_guard();
    let entries = semantic::backlinks(&vault.vault_ctx, &id, IndexOptions::default()).map_err(sem_err)?;
    serde_json::to_value(entries).map_err(failed)
}

pub fn search_text(vault: &VaultHandle, query: &str, limit: usize) -> OpResult {
    let needle = query.to_lowercase();
    if needle.trim().is_empty() {
        return Err(RpcError::failed("the search text is empty"));
    }
    let mut hits = Vec::new();
    for page in cerbo_core::page::page_list(&vault.page_ctx).map_err(failed)? {
        let content = object::object_read(&vault.page_ctx, &page.uuid).unwrap_or_default();
        let in_title = page.title.to_lowercase().contains(&needle);
        let body = content.to_lowercase();
        let Some(at) = body.find(&needle) else {
            if in_title {
                hits.push((0u8, json!({ "uuid": page.uuid, "title": page.title, "snippet": "" })));
            }
            continue;
        };
        let snippet = snippet_around(&content, at, needle.len());
        hits.push((u8::from(!in_title), json!({ "uuid": page.uuid, "title": page.title, "snippet": snippet })));
    }
    // Title matches first; the page list is already ordered by title.
    hits.sort_by_key(|(rank, _)| *rank);
    Ok(Value::Array(hits.into_iter().take(limit).map(|(_, v)| v).collect()))
}

/// About 80 characters of `content` around the byte offset `at`.
fn snippet_around(content: &str, at: usize, len: usize) -> String {
    // `at` indexes the lowercased text, whose byte offsets can differ from the
    // original's; fall back to the start when they do not land on boundaries.
    let from = content.char_indices().map(|(i, _)| i).rfind(|&i| i <= at.saturating_sub(40)).unwrap_or(0);
    let to = content
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= at.saturating_add(len).saturating_add(40))
        .unwrap_or(content.len());
    content[from..to].replace('\n', " ").trim().to_string()
}

pub fn search_semantic(
    vault: &VaultHandle,
    embed: &EmbedWorker,
    query: String,
    limit: usize,
    types: Vec<String>,
    annotated: Vec<String>,
) -> OpResult {
    let _gate = vault.index_guard();
    let options = SearchOptions { limit, types, annotated };
    let ctx = vault.vault_ctx.clone();
    let hits = embed
        .run(move |embedder| semantic::search(&ctx, &query, &options, embedder, IndexOptions::default()))
        .map_err(failed)?
        .map_err(sem_err)?;
    serde_json::to_value(hits).map_err(failed)
}

pub fn similar(
    vault: &VaultHandle,
    embed: &EmbedWorker,
    page: &str,
    limit: usize,
    types: Vec<String>,
    annotated: Vec<String>,
) -> OpResult {
    let id = page_id(page)?;
    let _gate = vault.index_guard();
    let options = SearchOptions { limit, types, annotated };
    let ctx = vault.vault_ctx.clone();
    let hits = embed
        .run(move |embedder| semantic::similar(&ctx, &id, &options, embedder, IndexOptions::default()))
        .map_err(failed)?
        .map_err(sem_err)?;
    serde_json::to_value(hits).map_err(failed)
}

pub fn sparql(vault: &VaultHandle, query: &str) -> OpResult {
    let _gate = vault.index_guard();
    let output = semantic::query(&vault.vault_ctx, query, IndexOptions::default()).map_err(sem_err)?;
    serde_json::from_str(&output.to_json()).map_err(failed)
}

// ── Writes ───────────────────────────────────────────────────────────────────
//
// Each returns the page it wrote, so the caller can attribute the file events.

/// A write's result and the page it touched.
pub struct Written {
    pub page: Option<String>,
    pub result: OpResult,
}

fn written(page: &str, result: Value) -> Written {
    Written { page: Some(page.to_string()), result: Ok(result) }
}

fn refused(error: RpcError) -> Written {
    Written { page: None, result: Err(error) }
}

pub fn page_create(vault: &VaultHandle, title: String, content: Option<String>, path: Option<String>) -> Written {
    let _gate = vault.index_guard();
    if title.trim().is_empty() {
        return refused(RpcError::failed("a page needs a title"));
    }
    let id = match cerbo_core::page::page_create(&vault.page_ctx, title) {
        Ok(id) => id,
        Err(e) => return refused(failed(e)),
    };
    let follow_up = (|| {
        if let Some(path) = path.filter(|p| !p.is_empty()) {
            cerbo_core::page::page_update_virtual_path(&vault.page_ctx, id.clone(), Some(path))?;
        }
        if let Some(content) = content {
            cerbo_core::page::page_write(&vault.page_ctx, id.clone(), content)?;
        }
        Ok::<(), String>(())
    })();
    match follow_up {
        Ok(()) => written(&id, json!({ "uuid": id })),
        // The page exists, empty or at the root; say what did not apply.
        Err(e) => Written { page: Some(id.clone()), result: Err(failed(format!("page {id} was created, but: {e}"))) },
    }
}

/// A fingerprint of a page's text, to notice that it changed.
fn fingerprint(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// A fingerprint as the `version` an agent reads and sends back.
fn version_text(fingerprint: u64) -> String {
    format!("{fingerprint:016x}")
}

/// The fingerprint a `version` stands for; `None` when it is not one.
pub fn parse_version(version: &str) -> Option<u64> {
    u64::from_str_radix(version, 16).ok().filter(|_| version.len() == 16)
}

/// The fingerprint of page `id` as it is now; none when it cannot be read.
pub fn page_fingerprint(vault: &VaultHandle, id: &str) -> Option<u64> {
    let id = page_id(id).ok()?;
    object::object_read(&vault.page_ctx, &id).ok().map(|text| fingerprint(&text))
}

pub fn page_edit(vault: &VaultHandle, id: &str, content: String) -> Written {
    page_edit_if(vault, id, content, None)
}

/// Replace a page's text. With `base`, the fingerprint of the text the new
/// text was written against, the edit is refused with a conflict when the page
/// is not that text any more: a person's edit is never silently overwritten.
pub fn page_edit_if(vault: &VaultHandle, id: &str, content: String, base: Option<u64>) -> Written {
    let _gate = vault.index_guard();
    let id = match existing(vault, id) {
        Ok(id) => id,
        Err(e) => return refused(e),
    };
    if let Some(base) = base {
        let now = object::object_read(&vault.page_ctx, &id).ok().map(|text| fingerprint(&text));
        if now != Some(base) {
            return refused(RpcError::new(
                cerbo_core::rpc::CODE_CONFLICT,
                format!(
                    "the page {id} changed after this edit was requested, so it was not applied and the page is \
                     unchanged; read the page again and make the edit again"
                ),
            ));
        }
    }
    match cerbo_core::page::page_write(&vault.page_ctx, id.clone(), content) {
        Ok(_) => written(&id, json!({ "uuid": id })),
        Err(e) => refused(failed(e)),
    }
}

pub fn page_rename(vault: &VaultHandle, id: &str, title: String) -> Written {
    let _gate = vault.index_guard();
    let id = match existing(vault, id) {
        Ok(id) => id,
        Err(e) => return refused(e),
    };
    if title.trim().is_empty() {
        return refused(RpcError::failed("a page needs a title"));
    }
    match cerbo_core::page::page_update_title(&vault.page_ctx, id.clone(), title) {
        Ok(()) => {
            semantic::sync_objects(&vault.vault_ctx, std::slice::from_ref(&id));
            written(&id, json!({ "uuid": id }))
        }
        Err(e) => refused(failed(e)),
    }
}

pub fn page_move(vault: &VaultHandle, id: &str, path: Option<String>) -> Written {
    let _gate = vault.index_guard();
    let id = match existing(vault, id) {
        Ok(id) => id,
        Err(e) => return refused(e),
    };
    match cerbo_core::page::page_update_virtual_path(&vault.page_ctx, id.clone(), path) {
        Ok(()) => written(&id, json!({ "uuid": id })),
        Err(e) => refused(failed(e)),
    }
}

pub fn page_trash(vault: &VaultHandle, id: &str) -> Written {
    let _gate = vault.index_guard();
    let id = match existing(vault, id) {
        Ok(id) => id,
        Err(e) => return refused(e),
    };
    match cerbo_core::page::page_delete(&vault.page_ctx, id.clone()) {
        Ok(()) => written(&id, json!({ "uuid": id, "trashed": true })),
        Err(e) => refused(failed(e)),
    }
}

pub fn link_add(vault: &VaultHandle, from: &str, to: &str) -> Written {
    let _gate = vault.index_guard();
    let from = match existing(vault, from) {
        Ok(id) => id,
        Err(e) => return refused(e),
    };
    let to = match existing(vault, to) {
        Ok(id) => id,
        Err(e) => return refused(e),
    };
    let content = match object::object_read(&vault.page_ctx, &from) {
        Ok(c) => c,
        Err(e) => return refused(failed(e)),
    };
    if live_links(vault, &content).contains(&to) {
        return written(&from, json!({ "uuid": from, "added": false }));
    }
    let title = meta_of(vault, &to).map(|m| m.title).unwrap_or_else(|_| to.clone());
    let separator = if content.is_empty() || content.ends_with('\n') { "" } else { "\n" };
    let updated = format!("{content}{separator}\n[{title}](cerbo://{to})\n");
    match cerbo_core::page::page_write(&vault.page_ctx, from.clone(), updated) {
        Ok(_) => written(&from, json!({ "uuid": from, "added": true })),
        Err(e) => refused(failed(e)),
    }
}

// ── Previews for approval ────────────────────────────────────────────────────

/// What a write would do, in words and (for text) as a diff.
pub struct Preview {
    pub page: Option<String>,
    pub summary: String,
    pub diff: Option<String>,
}

fn title_of(vault: &VaultHandle, id: &str) -> String {
    page_id(id)
        .ok()
        .and_then(|id| meta_of(vault, &id).ok())
        .map(|m| m.title)
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| id.to_string())
}

fn unified_diff(old: &str, new: &str) -> String {
    similar::TextDiff::from_lines(old, new).unified_diff().context_radius(3).to_string()
}

/// Describe `call` for a person to approve. Never fails: what cannot be read
/// is described by its id.
pub fn preview(vault: &VaultHandle, call: &cerbo_core::rpc::Call) -> Preview {
    use cerbo_core::rpc::Call;
    match call {
        Call::PageCreate { title, content, path, .. } => {
            let text = content.clone().unwrap_or_else(|| format!("# {title}\n"));
            let place = path.as_deref().filter(|p| !p.is_empty()).map(|p| format!(" in {p}")).unwrap_or_default();
            Preview {
                page: None,
                summary: format!("Create the page \"{title}\"{place}"),
                diff: Some(unified_diff("", &text)),
            }
        }
        Call::PageEdit { page, content, .. } => {
            let id = page_id(page).unwrap_or_else(|_| page.clone());
            let old = object::object_read(&vault.page_ctx, &id).unwrap_or_default();
            Preview {
                summary: format!("Edit the page \"{}\"", title_of(vault, &id)),
                diff: Some(unified_diff(&old, content)),
                page: Some(id),
            }
        }
        Call::PageRename { page, title, .. } => Preview {
            page: Some(page.clone()),
            summary: format!("Rename the page \"{}\" to \"{title}\"", title_of(vault, page)),
            diff: None,
        },
        Call::PageMove { page, path, .. } => Preview {
            page: Some(page.clone()),
            summary: format!(
                "Move the page \"{}\" to {}",
                title_of(vault, page),
                path.as_deref().filter(|p| !p.is_empty()).unwrap_or("the root")
            ),
            diff: None,
        },
        Call::PageTrash { page, .. } => Preview {
            page: Some(page.clone()),
            summary: format!("Move the page \"{}\" to the trash", title_of(vault, page)),
            diff: None,
        },
        Call::LinkAdd { from, to, .. } => Preview {
            page: Some(from.clone()),
            summary: format!("Add a link from \"{}\" to \"{}\"", title_of(vault, from), title_of(vault, to)),
            diff: None,
        },
        other => Preview { page: None, summary: format!("{other:?}"), diff: None },
    }
}
