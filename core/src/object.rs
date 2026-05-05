use crate::CerboContext;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

// ── Object Types ─────────────────────────────────────────────
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

    fn from_turtle(_content: &str) -> io::Result<Self> {
        // TODO: Parse Turtle format properly using rio_turtle
        // For now, return a placeholder
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Turtle parsing not yet implemented",
        ))
    }
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
