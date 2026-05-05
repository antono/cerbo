use crate::{CerboContext, index};
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

    // Create page.md for Page/Source/Ontology types
    if matches!(obj_type, ObjectType::Product | ObjectType::Source | ObjectType::Ontology) {
        let page_path = obj_dir.join("page.md");
        let content = format!("# {}\n", title);
        fs::write(&page_path, content)
            .map_err(|e| format!("Failed to write page.md: {}", e))?;
    }

    // Update index
    index::index_add(ctx, &title, &uuid)?;

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

/// Read page.md content for an object.
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
    fs::write(&page_path, content)
        .map_err(|e| format!("Failed to write page.md: {}", e))?;

    // Update modified timestamp
    // TODO: Update meta.ttl with new modified date

    // Extract links and annotations
    // TODO: Implement extraction of cerbo:// links and HackMD annotations

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CerboContext;
    use std::fs;

    fn create_test_context() -> CerboContext {
        let dir = std::env::temp_dir().join("cerbo_test");
        let _ = fs::create_dir_all(&dir);
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
        assert!(content.contains(":title \"Meta Test\""));
        assert!(content.contains("schema:dateCreated"));
        assert!(content.contains("schema:dateModified"));

        let _ = object_delete(&ctx, &uuid);
        cleanup(&ctx);
    }
}
