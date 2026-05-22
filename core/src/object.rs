use crate::{CerboContext, index};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

// ── Object Types ─────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    Product,   // User-created, editable
    Source,    // Imported, read-only
    Attachment, // Binary file
    Ontology,  // Ontology definition
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Product => "Product",
            ObjectType::Source => "Source",
            ObjectType::Attachment => "Attachment",
            ObjectType::Ontology => "Ontology",
        }
    }

    pub fn is_readonly(&self) -> bool {
        matches!(self, ObjectType::Source)
    }
}

// ── Object Metadata (meta.ttl) ──────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub object_type: ObjectType,
    pub title: String,
    pub created: String,
    pub modified: String,
    pub original_url: Option<String>,
    pub mime_type: Option<String>,
}

impl ObjectMeta {
    /// Write meta.ttl in Turtle RDF format
    pub fn write_to_file(&self, meta_path: &Path) -> io::Result<()> {
        let turtle = self.to_turtle();
        fs::write(meta_path, turtle)
    }

    /// Read meta.ttl from Turtle RDF format
    pub fn read_from_file(meta_path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(meta_path)?;
        Self::from_turtle(&content)
    }

    fn to_turtle(&self) -> String {
        let type_str = match self.object_type {
            ObjectType::Product => ":Product",
            ObjectType::Source => ":Source",
            ObjectType::Ontology => ":Ontology",
            ObjectType::Attachment => ":Attachment",
        };

        let mut lines = vec![
            "@prefix : <cerbo://ontology/> .".to_string(),
            "@prefix schema: <cerbo://ontology/schema/> .".to_string(),
            "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .".to_string(),
            "".to_string(),
            "<cerbo://objects/<uuid>>".to_string(),
            format!("    :type {} ;", type_str),
            format!("    :title \"{}\" ;", self.title),
            format!(
                "    schema:dateCreated \"{}\"^^xsd:dateTime ;",
                self.created
            ),
            format!(
                "    schema:dateModified \"{}\"^^xsd:dateTime .",
                self.modified
            ),
        ];

        if let Some(url) = &self.original_url {
            lines.push(format!("    :original-url \"{}\" .", url));
        }

        if let Some(mime) = &self.mime_type {
            lines.push(format!("    :mime-type \"{}\" .", mime));
        }

        lines.join("\n") + "\n"
    }

    fn from_turtle(content: &str) -> io::Result<Self> {
        // Simple parser for our generated Turtle format
        let mut object_type = ObjectType::Product;
        let mut title = String::new();
        let mut created = String::new();
        let mut modified = String::new();
        let mut original_url = None;
        let mut mime_type = None;

        for line in content.lines() {
            let line = line.trim();
            if line.contains(":type :") {
                if line.contains(":Product") {
                    object_type = ObjectType::Product;
                } else if line.contains(":Source") {
                    object_type = ObjectType::Source;
                } else if line.contains(":Ontology") {
                    object_type = ObjectType::Ontology;
                } else if line.contains(":Attachment") {
                    object_type = ObjectType::Attachment;
                }
            } else if line.contains(":title ") {
                title = extract_quoted_value(line);
            } else if line.contains("schema:dateCreated") {
                created = extract_value(line);
            } else if line.contains("schema:dateModified") {
                modified = extract_value(line);
            } else if line.contains(":original-url") {
                original_url = Some(extract_quoted_value(line));
            } else if line.contains(":mime-type") {
                mime_type = Some(extract_quoted_value(line));
            }
        }

        Ok(ObjectMeta {
            object_type,
            title,
            created,
            modified,
            original_url,
            mime_type,
        })
    }
}

// Helper functions for Turtle parsing
fn extract_quoted_value(line: &str) -> String {
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].find('"') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    String::new()
}

fn extract_value(line: &str) -> String {
    // Extract value after "^^xsd:dateTime" or similar
    if let Some(start) = line.find("\"") {
        if let Some(end) = line[start + 1..].find("\"") {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    String::new()
}

// ── Core Functions ────────────────────────────────────────────

pub fn objects_dir(ctx: &CerboContext) -> PathBuf {
    ctx.config_dir.join("objects")
}

pub fn object_path(ctx: &CerboContext, uuid: &str) -> PathBuf {
    objects_dir(ctx).join(uuid)
}

/// Create a new object with the given type and title.
/// Generates a new UUID if none provided.
pub fn object_create(
    ctx: &CerboContext,
    uuid: Option<String>,
    obj_type: ObjectType,
    title: String,
) -> Result<String, String> {
    let uuid = uuid.unwrap_or_else(|| Uuid::new_v4().to_string());
    let obj_dir = object_path(ctx, &uuid);

    fs::create_dir_all(&obj_dir).map_err(|e| format!("Failed to create object dir: {}", e))?;

    // Create meta.ttl
    let now = chrono::Utc::now().to_rfc3339();
    let meta = ObjectMeta {
        object_type: obj_type,
        title: title.clone(),
        created: now.clone(),
        modified: now,
        original_url: None,
        mime_type: None,
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // Create page.md for Product/Source/Ontology types (not Attachment)
    if !matches!(obj_type, ObjectType::Attachment) {
        let page_path = obj_dir.join("page.md");
        let content = format!("# {}\n", title);
        fs::write(&page_path, content)
            .map_err(|e| format!("Failed to write page.md: {}", e))?;
    }

    // Update index (best effort - don't fail if index update fails)
    let _ = index::index_add(ctx, &title, &uuid);

    Ok(uuid)
}

/// Delete an object by UUID. Fails if type is Source (read-only).
pub fn object_delete(ctx: &CerboContext, uuid: &str) -> Result<(), String> {
    let obj_dir = object_path(ctx, uuid);

    if !obj_dir.exists() {
        return Err(format!("Object not found: {}", uuid));
    }

    // Check if read-only
    let meta_path = obj_dir.join("meta.ttl");
    if meta_path.exists() {
        let meta = ObjectMeta::read_from_file(&meta_path)
            .map_err(|e| format!("Failed to read meta.ttl: {}", e))?;
        if meta.object_type.is_readonly() {
            return Err("Cannot delete source type (read-only)".to_string());
        }
    }

    fs::remove_dir_all(&obj_dir)
        .map_err(|e| format!("Failed to delete object: {}", e))?;

    Ok(())
}

/// Import a URL as a Source object (read-only)
/// Fetches content from URL and creates type: Source object
pub fn object_import(ctx: &CerboContext, url: &str) -> Result<String, String> {
    // Fetch content (use reqwest blocking client)
    let body = fetch_url_content(url)?;

    // Create Source object
    let uuid = Uuid::new_v4().to_string();
    let obj_dir = object_path(ctx, &uuid);

    fs::create_dir_all(&obj_dir).map_err(|e| format!("Failed to create object dir: {}", e))?;

    // Create meta.ttl with original-url
    let now = chrono::Utc::now().to_rfc3339();
    let meta = ObjectMeta {
        object_type: ObjectType::Source,
        title: format!("Imported: {}", url),
        created: now.clone(),
        modified: now,
        original_url: Some(url.to_string()),
        mime_type: Some("text/markdown".to_string()),
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // Create page.md with imported content
    let page_path = obj_dir.join("page.md");
    fs::write(&page_path, body)
        .map_err(|e| format!("Failed to write page.md: {}", e))?;

    // Update index
    let _ = index::index_add(ctx, &format!("Imported: {}", url), &uuid);

    Ok(uuid)
}

/// Import an ontology URL as an Ontology object
/// Fetches content and creates type: Ontology object
pub fn object_import_ontology(ctx: &CerboContext, url: &str) -> Result<String, String> {
    // Fetch content
    let body = fetch_url_content(url)?;

    // Create Ontology object
    let uuid = Uuid::new_v4().to_string();
    let obj_dir = object_path(ctx, &uuid);

    fs::create_dir_all(&obj_dir).map_err(|e| format!("Failed to create object dir: {}", e))?;

    // Create meta.ttl
    let now = chrono::Utc::now().to_rfc3339();
    let meta = ObjectMeta {
        object_type: ObjectType::Ontology,
        title: format!("Ontology: {}", url),
        created: now.clone(),
        modified: now,
        original_url: Some(url.to_string()),
        mime_type: Some("text/markdown".to_string()),
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // Create page.md with ontology content
    let page_path = obj_dir.join("page.md");
    fs::write(&page_path, body)
        .map_err(|e| format!("Failed to write page.md: {}", e))?;

    // Update index
    let _ = index::index_add(ctx, &format!("Ontology: {}", url), &uuid);

    // Update ontology-map.json with prefix→uuid mapping
    update_ontology_map(ctx, &uuid, url)?;

    Ok(uuid)
}

/// Update ontology-map.json with new prefix→UUID mapping
fn update_ontology_map(ctx: &CerboContext, uuid: &str, url: &str) -> Result<(), String> {
    let map_path = ctx.config_dir.join("ontology-map.json");
    
    let mut map: std::collections::HashMap<String, String> = if map_path.exists() {
        let content = fs::read_to_string(&map_path)
            .map_err(|e| format!("Failed to read ontology-map.json: {}", e))?;
        
        // Handle both {"prefixes": {}} and actual map formats
        if content.trim().starts_with("{") {
            // Try to parse as a map directly
            match serde_json::from_str::<std::collections::HashMap<String, String>>(&content) {
                Ok(m) => m,
                Err(_) => {
                    // Try to parse as {"prefixes": {...}}
                    #[derive(Serialize, Deserialize)]
                    struct OntologyMap { prefixes: std::collections::HashMap<String, String> }
                    match serde_json::from_str::<OntologyMap>(&content) {
                        Ok(om) => om.prefixes,
                        Err(e) => return Err(format!("Failed to parse ontology-map.json: {}", e)),
                    }
                }
            }
        } else {
            std::collections::HashMap::new()
        }
    } else {
        std::collections::HashMap::new()
    };

    // Extract prefix from URL (e.g., "schema.org" → "schema")
    let prefix = extract_prefix_from_url(url);
    map.insert(prefix, uuid.to_string());

    // Write back in {"prefixes": {...}} format
    #[derive(Serialize)]
    struct OntologyMap<'a> { prefixes: &'a std::collections::HashMap<String, String> }
    let om = OntologyMap { prefixes: &map };
    
    let content = serde_json::to_string_pretty(&om)
        .map_err(|e| format!("Failed to serialize ontology-map.json: {}", e))?;
    
    fs::write(&map_path, content)
        .map_err(|e| format!("Failed to write ontology-map.json: {}", e))
}

/// Extract prefix from ontology URL
fn extract_prefix_from_url(url: &str) -> String {
    // Simple extraction: get last part of domain
    // e.g., "https://schema.org/" → "schema"
    // e.g., "https://xmlns.com/foaf/0.1/" → "foaf"
    if let Some(domain) = url.split("://").nth(1) {
        let domain = domain.split('/').next().unwrap_or(domain);
        if let Some(name) = domain.split('.').nth(0) {
            return name.to_lowercase();
        }
    }
    "unknown".to_string()
}

/// Fetch URL content using curl (more compatible with tokio)
fn fetch_url_content(url: &str) -> Result<String, String> {
    let output = std::process::Command::new("curl")
        .arg("-s")  // silent mode
        .arg("-L")  // follow redirects
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to execute curl: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl error for {}: {}", url, stderr));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in response: {}", e))
}
pub fn object_read(ctx: &CerboContext, uuid: &str) -> Result<String, String> {
    let obj_dir = object_path(ctx, uuid);
    let page_path = obj_dir.join("page.md");

    fs::read_to_string(&page_path)
        .map_err(|e| format!("Failed to read page.md: {}", e))
}

/// Write page.md content for an object. Fails if type is Source (read-only).
pub fn object_write(ctx: &CerboContext, uuid: &str, content: &str) -> Result<(), String> {
    let obj_dir = object_path(ctx, uuid);

    if !obj_dir.exists() {
        return Err(format!("Object not found: {}", uuid));
    }

    // Check if read-only
    let meta_path = obj_dir.join("meta.ttl");
    if meta_path.exists() {
        let meta = ObjectMeta::read_from_file(&meta_path)
            .map_err(|e| format!("Failed to read meta.ttl: {}", e))?;
        if meta.object_type.is_readonly() {
            return Err("Cannot write to source type (read-only)".to_string());
        }
    }

    let page_path = obj_dir.join("page.md");
    let old_content = fs::read_to_string(&page_path).unwrap_or_default();

    fs::write(&page_path, content)
        .map_err(|e| format!("Failed to write page.md: {}", e))?;

    // Update modified timestamp in meta.ttl
    update_modified_date(&meta_path)?;

    // Extract links and update backrefs.ttl
    update_backrefs(ctx, uuid, &old_content, content);

    Ok(())
}

/// Update modified date in meta.ttl
fn update_modified_date(meta_path: &PathBuf) -> Result<(), String> {
    if !meta_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(meta_path)
        .map_err(|e| format!("Failed to read meta.ttl: {}", e))?;

    let now = chrono::Utc::now().to_rfc3339();
    let re = Regex::new(r#"schema:dateModified\s+"[^"]*""#).unwrap();
    let new_content = re.replace(&content, format!("schema:dateModified \"{}\"", now));
    
    // Convert Cow<str> to String properly
    let new_content_string: String = new_content.into_owned();
    
    if new_content_string != content {
        fs::write(meta_path, new_content_string)
            .map_err(|e| format!("Failed to update meta.ttl: {}", e))?;
    }

    Ok(())
}

/// Update backrefs.ttl based on old and new content
fn update_backrefs(ctx: &CerboContext, source_uuid: &str, old_content: &str, new_content: &str) {
    // Extract links from old and new content
    let old_links = crate::links::extract_cerbo_links(old_content);
    let new_links = crate::links::extract_cerbo_links(new_content);

    // Remove backrefs for links that were removed
    for target_uuid in &old_links {
        if !new_links.contains(target_uuid) {
            let _ = crate::links::backrefs_remove(ctx, target_uuid, source_uuid);
        }
    }

    // Add backrefs for new links
    for target_uuid in &new_links {
        if !old_links.contains(target_uuid) {
            let _ = crate::links::backrefs_add(ctx, target_uuid, source_uuid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CerboContext;
    use std::fs;

    fn create_test_context() -> CerboContext {
        let dir = std::env::temp_dir().join("cerbo_test");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(dir.join("objects"));
        CerboContext {
            config_dir: dir.clone(),
            cache_dir: dir.join("cache"),
        }
    }

    fn cleanup(ctx: &CerboContext) {
        let _ = fs::remove_dir_all(&ctx.config_dir);
    }

    #[test]
    fn test_object_create_and_read() {
        let ctx = create_test_context();

        // Create a Product page
        let uuid = object_create(&ctx, None, ObjectType::Product, "Test Page".to_string()).unwrap();
        assert!(!uuid.is_empty());

        // Read page.md
        let content = object_read(&ctx, &uuid).unwrap();
        assert!(content.contains("# Test Page"));

        // Cleanup
        let _ = object_delete(&ctx, &uuid);
        cleanup(&ctx);
    }

    #[test]
    fn test_object_create_with_specific_uuid() {
        let ctx = create_test_context();

        let test_uuid = "test-uuid-1234";
        let uuid = object_create(&ctx, Some(test_uuid.to_string()), ObjectType::Product, "Another Page".to_string()).unwrap();
        assert_eq!(uuid, test_uuid);

        let _ = object_delete(&ctx, test_uuid);
        cleanup(&ctx);
    }

    #[test]
    fn test_object_read_nonexistent() {
        let ctx = create_test_context();

        let result = object_read(&ctx, "non-existent-uuid");
        assert!(result.is_err());

        cleanup(&ctx);
    }

    #[test]
    fn test_object_delete_nonexistent() {
        let ctx = create_test_context();

        let result = object_delete(&ctx, "non-existent-uuid");
        assert!(result.is_err());

        cleanup(&ctx);
    }

    #[test]
    fn test_source_type_readonly() {
        let ctx = create_test_context();

        // Create a Source object (read-only)
        let uuid = object_create(&ctx, None, ObjectType::Source, "Imported Page".to_string()).unwrap();

        // Try to write (should fail)
        let result = object_write(&ctx, &uuid, "new content");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-only"));

        // Try to delete (should fail)
        let result = object_delete(&ctx, &uuid);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-only"));

        // Cleanup: manually remove for test (bypass read-only check)
        let obj_dir = object_path(&ctx, &uuid);
        let _ = fs::remove_dir_all(&obj_dir);

        cleanup(&ctx);
    }

    #[test]
    fn test_object_write_and_read() {
        let ctx = create_test_context();

        let uuid = object_create(&ctx, None, ObjectType::Product, "Editable Page".to_string()).unwrap();

        // Write new content
        let new_content = "# Editable Page\n\nThis is updated content.";
        object_write(&ctx, &uuid, new_content).unwrap();

        // Read back
        let content = object_read(&ctx, &uuid).unwrap();
        assert!(content.contains("This is updated content."));

        let _ = object_delete(&ctx, &uuid);
        cleanup(&ctx);
    }

    #[test]
    fn test_object_type_attachment() {
        let ctx = create_test_context();

        // Create an Attachment object (no page.md)
        let uuid = object_create(&ctx, None, ObjectType::Attachment, "image.png".to_string()).unwrap();

        let obj_dir = object_path(&ctx, &uuid);
        assert!(obj_dir.exists());
        assert!(!obj_dir.join("page.md").exists()); // No page.md for attachments

        let _ = object_delete(&ctx, &uuid);
        cleanup(&ctx);
    }

    #[test]
    fn test_object_type_ontology() {
        let ctx = create_test_context();

        // Create an Ontology object
        let uuid = object_create(&ctx, None, ObjectType::Ontology, "Schema.org".to_string()).unwrap();

        let obj_dir = object_path(&ctx, &uuid);
        assert!(obj_dir.join("page.md").exists()); // Has page.md
        assert!(obj_dir.join("meta.ttl").exists());

        let _ = object_delete(&ctx, &uuid);
        cleanup(&ctx);
    }

    #[test]
    fn test_meta_ttl_creation() {
        let ctx = create_test_context();

        let uuid = object_create(&ctx, None, ObjectType::Product, "Meta Test".to_string()).unwrap();

        let obj_dir = object_path(&ctx, &uuid);
        let meta_path = obj_dir.join("meta.ttl");
        assert!(meta_path.exists());

        let content = fs::read_to_string(&meta_path).unwrap();
        assert!(content.contains(":type :Product"));
        assert!(content.contains(r#":title "Meta Test""#));
        assert!(content.contains("schema:dateModified"));

        let _ = object_delete(&ctx, &uuid);
        cleanup(&ctx);
    }
}

// ── Attachment Management ──────────────────────────────────────

/// Add an attachment to a page.
/// Creates a type: Attachment object, copies the file, returns UUID.
pub fn attachment_add(ctx: &CerboContext, _page_uuid: &str, file_path: &std::path::Path) -> Result<String, String> {
    // Create Attachment object
    let uuid = Uuid::new_v4().to_string();
    let obj_dir = object_path(ctx, &uuid);

    fs::create_dir_all(&obj_dir).map_err(|e| format!("Failed to create object dir: {}", e))?;

    // Copy file to object directory
    let file_name = file_path
        .file_name()
        .ok_or("Invalid file path".to_string())?
        .to_string_lossy()
        .to_string();
    let dest_path = obj_dir.join(&file_name);

    fs::copy(file_path, &dest_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    // Detect MIME type from file extension
    let mime_type = detect_mime_type(file_path);

    // Create meta.ttl
    let now = chrono::Utc::now().to_rfc3339();
    let meta = ObjectMeta {
        object_type: ObjectType::Attachment,
        title: file_name.clone(),
        created: now.clone(),
        modified: now,
        original_url: None,
        mime_type: Some(mime_type),
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // No page.md for attachments

    // Update index
    let _ = index::index_add(ctx, &file_name, &uuid);

    Ok(uuid)
}

/// Delete an attachment object.
pub fn attachment_delete(ctx: &CerboContext, attachment_uuid: &str) -> Result<(), String> {
    object_delete(ctx, attachment_uuid)
}

/// List attachments for a page (via backrefs.ttl :usesAttachment).
pub fn attachment_list(ctx: &CerboContext, page_uuid: &str) -> Result<Vec<String>, String> {
    // Read page's backrefs.ttl for :usesAttachment
    let backrefs = crate::links::backrefs_read(ctx, page_uuid)?;

    // Filter for attachments (objects of type Attachment)
    let mut attachments = Vec::new();
    for uuid in backrefs {
        let obj_dir = object_path(ctx, &uuid);
        let meta_path = obj_dir.join("meta.ttl");
        if meta_path.exists() {
            let meta = ObjectMeta::read_from_file(&meta_path)
                .map_err(|e| format!("Failed to read meta.ttl: {}", e))?;
            if matches!(meta.object_type, ObjectType::Attachment) {
                attachments.push(uuid);
            }
        }
    }

    Ok(attachments)
}

/// Detect MIME type from file extension.
fn detect_mime_type(file_path: &std::path::Path) -> String {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "png" => "image/png".to_string(),
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "gif" => "image/gif".to_string(),
        "pdf" => "application/pdf".to_string(),
        "txt" => "text/plain".to_string(),
        "md" => "text/markdown".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}
