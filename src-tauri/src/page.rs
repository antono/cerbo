use crate::get_vault_ctx;
use cerbo_core::cursor_positions::{self, CursorPosition};
use cerbo_core::object;
use cerbo_core::CerboContext;
use tauri::AppHandle;

/// Run a command that changes vault files off the main thread. Each such
/// change also updates the semantic index, which can wait up to the index
/// lock's timeout while e.g. "Enable" holds it; on the main thread that wait
/// would freeze the window (spec: the similar notes panel never blocks editing).
/// The frontend sends these one at a time, so saves keep their order.
async fn off_main<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_create(app: AppHandle, title: String, slug: Option<String>, virtualPath: Option<String>) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_create_with_metadata(&ctx, title, slug, virtualPath)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_read(app: AppHandle, uuid: String) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    cerbo_core::page::page_read(&ctx, uuid)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_write(
    app: AppHandle,
    uuid: String,
    content: String,
) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_write(&ctx, uuid, content)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_update_title(
    app: AppHandle,
    uuid: String,
    newTitle: String,
) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_update_title(&ctx, uuid, newTitle)).await
}

/// Set or clear a page's `cerbo:virtualPath` (spec `page-virtual-path`).
#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_move(
    app: AppHandle,
    uuid: String,
    virtualPath: Option<String>,
) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_update_virtual_path(&ctx, uuid, virtualPath)).await
}

/// Rename a folder prefix on every page under it; the report lists the pages
/// changed and those that failed, each with its error.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn folder_rename(
    app: AppHandle,
    oldPrefix: String,
    newPrefix: String,
) -> Result<cerbo_core::page::FolderRenameReport, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::folder_rename(&ctx, &oldPrefix, &newPrefix)).await
}

/// The vault's sidebar layout from `.cerbo/tree.toml` (spec `vault-tree-layout`).
#[tauri::command]
pub async fn tree_layout(app: AppHandle) -> Result<cerbo_core::tree::TreeLayout, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::tree::tree_layout(&ctx)).await
}

/// Create an empty (pinned) folder.
#[tauri::command]
pub async fn folder_create(app: AppHandle, path: String) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::tree::folder_create(&ctx, &path)).await
}

/// Delete a folder with no pages under it.
#[tauri::command]
pub async fn folder_delete(app: AppHandle, path: String) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::tree::folder_delete(&ctx, &path)).await
}

/// Store the order of a folder's children (`""` is the root); returns the
/// list as stored, after pruning.
#[tauri::command]
pub async fn folder_set_order(app: AppHandle, parent: String, entries: Vec<String>) -> Result<Vec<String>, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::tree::folder_set_order(&ctx, &parent, entries)).await
}

/// Remember where editing a page ended (spec `editor-position-restore`).
/// Stored in `cursor.toml` in the app config dir, keyed by `vaultId` as sent
/// by the frontend rather than by the active vault: on a vault switch the page
/// being left belongs to the previous vault.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn cursor_position_save(
    app: AppHandle,
    vaultId: String,
    uuid: String,
    line: u32,
    column: u32,
    scrollTop: Option<f64>,
) -> Result<(), String> {
    let ctx = crate::get_context(&app)?;
    let position = CursorPosition { line, column, scroll_top: scrollTop.unwrap_or(0.0) };
    off_main(move || save_cursor(&ctx, &vaultId, &uuid, position)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn cursor_position_load(
    app: AppHandle,
    vaultId: String,
    uuid: String,
) -> Result<Option<CursorPosition>, String> {
    let ctx = crate::get_context(&app)?;
    // Off the main thread: it waits for the file lock of a running save.
    off_main(move || cursor_positions::load_cursor_position(&ctx, &vaultId, &uuid)).await
}

/// Save a position, dropping stored positions of the vault's deleted pages.
/// An unknown vault keeps every entry.
fn save_cursor(ctx: &CerboContext, vault_id: &str, uuid: &str, position: CursorPosition) -> Result<(), String> {
    let objects = cerbo_core::vault::get_vault_path(ctx, vault_id).map(|p| p.join(".cerbo").join("objects"));
    cursor_positions::save_cursor_position(ctx, vault_id, uuid, position, |page| {
        objects.as_ref().is_none_or(|dir| dir.join(page).exists())
    })
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_delete(app: AppHandle, uuid: String) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_delete(&ctx, uuid)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_list(app: AppHandle) -> Result<Vec<cerbo_core::page::PageMeta>, String> {
    let ctx = get_vault_ctx(&app)?;
    cerbo_core::page::page_list(&ctx)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_list(
    app: AppHandle,
    uuid: String,
) -> Result<Vec<String>, String> {
    let ctx = get_vault_ctx(&app)?;
    object::attachment_list(&ctx, &uuid)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn attachment_add(
    app: AppHandle,
    uuid: String,
    srcPath: std::path::PathBuf,
) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || object::attachment_add(&ctx, &uuid, &srcPath)).await
}

/// Header carrying the percent-encoded file name of an `attachment_upload`;
/// the body is the raw bytes, so there is no room for named arguments.
const UPLOAD_NAME_HEADER: &str = "x-cerbo-filename";

/// Store a dropped or pasted fragment as a new Attachment object (spec
/// `editor-attachment-drop`). The frontend invokes it with a `Uint8Array`
/// body, which arrives as `InvokeBody::Raw` without a JSON round trip, and the
/// file name in [`UPLOAD_NAME_HEADER`]. Returns the new object's UUID.
#[tauri::command]
pub async fn attachment_upload(app: AppHandle, request: tauri::ipc::Request<'_>) -> Result<String, String> {
    let header = request
        .headers()
        .get(UPLOAD_NAME_HEADER)
        .ok_or_else(|| format!("Missing {} header", UPLOAD_NAME_HEADER))?
        .to_str()
        .map_err(|e| format!("Invalid {} header: {}", UPLOAD_NAME_HEADER, e))?;
    let name = percent_decode(header)?;
    let bytes = match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => bytes.clone(),
        tauri::ipc::InvokeBody::Json(_) => {
            return Err(format!("Upload of {} must send raw bytes", name));
        }
    };
    let ctx = get_vault_ctx(&app)?;
    off_main(move || object::attachment_add_bytes(&ctx, &name, &bytes)).await
}

/// A web URL downloaded into a new Attachment object.
#[derive(serde::Serialize)]
pub struct FetchedAttachment {
    pub uuid: String,
    /// The file name it was stored under, which is also its title.
    pub name: String,
}

/// Largest download `attachment_fetch` stores.
const FETCH_LIMIT: u64 = 256 * 1024 * 1024;

/// Download an `http(s)` URL dropped on the editor (an image dragged out of a
/// web page, or a link wry reports as a dropped path) and store it as a new
/// Attachment object. The name comes from the URL's last path segment, with
/// the extension its `Content-Type` implies, else `fallbackName`.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn attachment_fetch(app: AppHandle, url: String, fallbackName: String) -> Result<FetchedAttachment, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || {
        let (bytes, content_type) = download(&url)?;
        let name = fetched_file_name(&url, content_type.as_deref(), &fallbackName);
        let uuid = object::attachment_add_bytes(&ctx, &name, &bytes)?;
        Ok(FetchedAttachment { uuid, name })
    })
    .await
}

/// Body and `Content-Type` of an `http(s)` URL, at most [`FETCH_LIMIT`] bytes.
fn download(url: &str) -> Result<(Vec<u8>, Option<String>), String> {
    use std::time::Duration;

    let scheme = url.split_once("://").map(|(s, _)| s.to_ascii_lowercase());
    if !matches!(scheme.as_deref(), Some("http" | "https")) {
        return Err(format!("Not a web URL: {}", url));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(30)))
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();
    let response = agent.get(url).call().map_err(|e| format!("{url}: {e}"))?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = response
        .into_body()
        .into_with_config()
        .limit(FETCH_LIMIT)
        .read_to_vec()
        .map_err(|e| format!("{url}: {e}"))?;
    Ok((bytes, content_type))
}

/// File extension for the `Content-Type`s a dropped URL usually serves.
fn extension_for(content_type: &str) -> Option<&'static str> {
    let mime = content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    Some(match mime.as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/bmp" => "bmp",
        "text/html" => "html",
        "text/plain" => "txt",
        "application/pdf" => "pdf",
        _ => return None,
    })
}

/// Name to store a downloaded URL under: its last path segment, decoded and
/// without separators, plus the extension `content_type` implies when the
/// segment lacks it; `fallback` when the URL has no usable segment.
fn fetched_file_name(url: &str, content_type: Option<&str>, fallback: &str) -> String {
    let path = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = path.split(['?', '#']).next().unwrap_or("");
    // The first segment is the host.
    let segment = path.split('/').skip(1).filter(|s| !s.is_empty()).last().unwrap_or("");
    let decoded = percent_decode(segment).unwrap_or_else(|_| segment.to_string());
    let cleaned: String = decoded
        .chars()
        .map(|c| if matches!(c, '/' | '\\') || c.is_control() { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim();
    let base = if cleaned.is_empty() || cleaned == "." || cleaned == ".." { fallback } else { cleaned };

    let Some(ext) = content_type.and_then(extension_for) else {
        return base.to_string();
    };
    let current = base.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match current.as_deref() {
        Some(e) if e == ext || (ext == "jpg" && e == "jpeg") => base.to_string(),
        _ => format!("{base}.{ext}"),
    }
}

/// Local paths of the files a file manager put on the clipboard. The paste
/// fallback for when the webview's `paste` event exposes no `text/uri-list` /
/// `x-special/gnome-copied-files` (design Decision 4, task 4.6). GTK's
/// clipboard can only be read on the main thread; empty off Linux.
#[tauri::command]
pub async fn clipboard_file_paths(app: AppHandle) -> Result<Vec<String>, String> {
    #[cfg(target_os = "linux")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(linux_clipboard_uris());
        })
        .map_err(|e| e.to_string())?;
        let uris = tauri::async_runtime::spawn_blocking(move || rx.recv().map_err(|e| e.to_string()))
            .await
            .map_err(|e| e.to_string())??;
        Ok(uris.iter().filter_map(|uri| file_uri_to_path(uri)).collect())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Ok(Vec::new())
    }
}

/// `text/uri-list` of the clipboard, else its `x-special/gnome-copied-files`
/// (`copy`/`cut`, then one URI per line).
#[cfg(target_os = "linux")]
fn linux_clipboard_uris() -> Vec<String> {
    let clipboard = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
    let uris: Vec<String> = clipboard.wait_for_uris().iter().map(|u| u.to_string()).collect();
    if !uris.is_empty() {
        return uris;
    }
    let target = gtk::gdk::Atom::intern("x-special/gnome-copied-files");
    clipboard
        .wait_for_contents(&target)
        .map(|data| {
            String::from_utf8_lossy(&data.data())
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && *line != "copy" && *line != "cut")
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Local path of a `file://` URI (`file:///…` or `file://localhost/…`).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn file_uri_to_path(uri: &str) -> Option<String> {
    let rest = uri.trim().strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    percent_decode(path).ok()
}

/// Decode `encodeURIComponent` output: `%XX` escapes to bytes, read as UTF-8.
fn percent_decode(encoded: &str) -> Result<String, String> {
    let raw = encoded.as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' {
            let hex = encoded
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| format!("Invalid escape in file name: {}", encoded))?;
            out.push(hex);
            i += 3;
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| format!("File name is not UTF-8: {}", encoded))
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn attachment_delete(
    app: AppHandle,
    uuid: String,
) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || object::attachment_delete(&ctx, &uuid)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_open(
    app: AppHandle,
    uuid: String,
    filename: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let ctx = get_vault_ctx(&app)?;
    let obj_dir = object::object_path(&ctx, &uuid);
    let path = obj_dir.join(&filename);
    app.opener()
        .open_path(path.to_string_lossy(), None::<String>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn cursor_positions_round_trip_and_drop_deleted_pages() {
        let tmp = TempDir::new().unwrap();
        let ctx = CerboContext { config_dir: tmp.path().join("config"), cache_dir: tmp.path().join("cache") };
        let vault = tmp.path().join("vault");
        std::fs::create_dir_all(vault.join(".cerbo/objects/page-a")).unwrap();
        std::fs::create_dir_all(vault.join(".cerbo/objects/page-b")).unwrap();
        let vault_id = cerbo_core::vault::vault_add(&ctx, "Vault".into(), vault.to_string_lossy().into_owned()).unwrap().id;

        let a = CursorPosition { line: 137, column: 3, scroll_top: 1370.0 };
        let b = CursorPosition { line: 2, column: 1, scroll_top: 0.0 };
        save_cursor(&ctx, &vault_id, "page-a", a).unwrap();
        save_cursor(&ctx, &vault_id, "page-b", b).unwrap();
        assert_eq!(cursor_positions::load_cursor_position(&ctx, &vault_id, "page-a").unwrap(), Some(a));
        assert_eq!(cursor_positions::load_cursor_position(&ctx, &vault_id, "page-b").unwrap(), Some(b));

        // The next first save of a page drops the deleted page's entry.
        std::fs::remove_dir_all(vault.join(".cerbo/objects/page-a")).unwrap();
        std::fs::create_dir_all(vault.join(".cerbo/objects/page-c")).unwrap();
        save_cursor(&ctx, &vault_id, "page-c", b).unwrap();
        assert_eq!(cursor_positions::load_cursor_position(&ctx, &vault_id, "page-a").unwrap(), None);
    }

    #[test]
    fn file_uri_to_path_decodes_local_uris_only() {
        assert_eq!(file_uri_to_path("file:///home/u/My%20Report%20137.pdf").as_deref(), Some("/home/u/My Report 137.pdf"));
        assert_eq!(file_uri_to_path("file://localhost/tmp/a.png").as_deref(), Some("/tmp/a.png"));
        assert_eq!(file_uri_to_path("file://otherhost/a.png"), None);
        assert_eq!(file_uri_to_path("https://example.org/a.png"), None);
    }

    #[test]
    fn fetched_file_name_uses_the_url_and_content_type() {
        assert_eq!(fetched_file_name("https://example.org/img/photo.png?w=137", Some("image/png"), "x"), "photo.png");
        assert_eq!(fetched_file_name("https://example.org/a/photo.jpeg", Some("image/jpeg"), "x"), "photo.jpeg");
        assert_eq!(fetched_file_name("https://example.org/render?id=137", Some("image/webp"), "x"), "render.webp");
        assert_eq!(fetched_file_name("https://example.org/My%20Cat.gif", None, "x"), "My Cat.gif");
        assert_eq!(fetched_file_name("https://example.org/a%2F..%2Fb.png", None, "x"), "a_.._b.png");
    }

    #[test]
    fn fetched_file_name_falls_back_without_a_path() {
        assert_eq!(
            fetched_file_name("https://example.org/", Some("text/html; charset=utf-8"), "dropped-20261107-131415"),
            "dropped-20261107-131415.html"
        );
        assert_eq!(fetched_file_name("https://example.org", None, "dropped"), "dropped");
        assert_eq!(fetched_file_name("https://example.org/..", Some("image/png"), "dropped"), "dropped.png");
    }

    #[test]
    fn percent_decode_reverses_encode_uri_component() {
        // encodeURIComponent("Отчёт 137%.pdf")
        assert_eq!(
            percent_decode("%D0%9E%D1%82%D1%87%D1%91%D1%82%20137%25.pdf").unwrap(),
            "Отчёт 137%.pdf"
        );
        assert_eq!(percent_decode("photo.png").unwrap(), "photo.png");
    }

    #[test]
    fn percent_decode_rejects_broken_escapes() {
        assert!(percent_decode("bad%2").is_err());
        assert!(percent_decode("bad%zz").is_err());
        assert!(percent_decode("%FF").is_err());
    }
}
