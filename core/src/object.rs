use crate::CerboContext;
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

    /// The inverse of [`ObjectType::as_str`]: the local name of a `cerbo:` type IRI.
    pub fn from_local(local: &str) -> Option<Self> {
        match local {
            "Product" => Some(ObjectType::Product),
            "Source" => Some(ObjectType::Source),
            "Attachment" => Some(ObjectType::Attachment),
            "Ontology" => Some(ObjectType::Ontology),
            _ => None,
        }
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
    pub slug: Option<String>,
    pub virtual_path: Option<String>,
}

/// Validate a slug: ASCII alphanumeric + dash, no leading/trailing dash, len ≤ 80, non-empty.
pub fn is_valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && !s.contains('/')
        && !s.starts_with('-')
        && !s.ends_with('-')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

impl ObjectMeta {
    /// Write `meta.ttl` for the object with this UUID.
    pub fn write_to_file(&self, meta_path: &Path, uuid: &str) -> io::Result<()> {
        crate::fsio::write_atomic_str(meta_path, &self.to_turtle(uuid))
    }

    /// Read `meta.ttl` from Turtle.
    pub fn read_from_file(meta_path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(meta_path)?;
        Self::from_turtle(&content)
    }

    /// Serialise as Turtle, with the object's own IRI as the subject.
    pub fn to_turtle(&self, uuid: &str) -> String {
        use crate::rdf;
        use oxrdf::{Literal, Term, Triple};

        let subject = rdf::object_iri(uuid);
        let text = |v: &str| Term::from(Literal::new_simple_literal(v));

        let mut triples = vec![
            Triple::new(
                subject.clone(),
                rdf::cerbo("type"),
                Term::from(rdf::cerbo(self.object_type.as_str())),
            ),
            Triple::new(subject.clone(), rdf::cerbo("title"), text(&self.title)),
            Triple::new(
                subject.clone(),
                rdf::schema("dateCreated"),
                Term::from(rdf::date_time(&self.created)),
            ),
            Triple::new(
                subject.clone(),
                rdf::schema("dateModified"),
                Term::from(rdf::date_time(&self.modified)),
            ),
        ];

        // Optional fields keep a fixed order so an unchanged rewrite is byte-stable.
        for (local, value) in [
            ("original-url", self.original_url.as_deref()),
            ("mime-type", self.mime_type.as_deref()),
            ("slug", self.slug.as_deref()),
            ("virtualPath", self.virtual_path.as_deref()),
        ] {
            if let Some(v) = value {
                triples.push(Triple::new(subject.clone(), rdf::cerbo(local), text(v)));
            }
        }

        rdf::serialize(&triples)
    }

    fn from_turtle(content: &str) -> io::Result<Self> {
        match crate::rdf::parse(content) {
            Ok(triples) => Ok(Self::from_triples(&triples)),
            // Vaults written before this change hold `meta.ttl` that is not valid
            // Turtle: `cerbo:` was used without being declared, and the optional
            // predicates dangle after a statement terminator. Those files must keep
            // opening; they are rewritten valid on the object's next save.
            Err(_) => Ok(Self::from_legacy_lines(content)),
        }
    }

    fn empty() -> Self {
        ObjectMeta {
            object_type: ObjectType::Product,
            title: String::new(),
            created: String::new(),
            modified: String::new(),
            original_url: None,
            mime_type: None,
            slug: None,
            virtual_path: None,
        }
    }

    /// Interpret parsed triples. A value is only ever read as a value, so no title
    /// can present itself as a type.
    fn from_triples(triples: &[oxrdf::Triple]) -> Self {
        use oxrdf::Term;

        let mut meta = Self::empty();

        for triple in triples {
            let Some((ns, local)) = split_namespace(triple.predicate.as_str()) else {
                continue;
            };
            let literal = match &triple.object {
                Term::Literal(l) => Some(l.value()),
                _ => None,
            };

            match (ns, local) {
                (Namespace::Cerbo, "type") => {
                    if let Term::NamedNode(n) = &triple.object
                        && let Some((Namespace::Cerbo, name)) = split_namespace(n.as_str())
                        && let Some(t) = ObjectType::from_local(name)
                    {
                        meta.object_type = t;
                    }
                }
                (Namespace::Cerbo, "title") => {
                    if let Some(v) = literal {
                        meta.title = v.to_string();
                    }
                }
                (Namespace::Cerbo, "original-url") => {
                    meta.original_url = literal.map(str::to_string);
                }
                (Namespace::Cerbo, "mime-type") => {
                    meta.mime_type = literal.map(str::to_string);
                }
                (Namespace::Cerbo, "slug") => {
                    meta.slug = literal.filter(|v| is_valid_slug(v)).map(str::to_string);
                }
                (Namespace::Cerbo, "virtualPath") => {
                    meta.virtual_path = literal
                        .filter(|v| crate::vault::validate_virtual_path(v).is_ok())
                        .map(str::to_string);
                }
                (Namespace::Schema, "dateCreated") => {
                    if let Some(v) = literal {
                        meta.created = v.to_string();
                    }
                }
                (Namespace::Schema, "dateModified") => {
                    if let Some(v) = literal {
                        meta.modified = v.to_string();
                    }
                }
                _ => {}
            }
        }

        meta
    }

    /// Reader for `meta.ttl` written before this change, which is not valid Turtle.
    ///
    /// The predicate is matched at the start of the line, never anywhere inside it,
    /// so a title like `RDF :type :Source notes` stays a title.
    fn from_legacy_lines(content: &str) -> Self {
        let mut meta = Self::empty();

        for line in content.lines() {
            let line = line.trim();
            let Some((predicate, rest)) = line.split_once(char::is_whitespace) else {
                continue;
            };
            let Some(local) = legacy_local_name(predicate) else {
                continue;
            };

            match local {
                "type" => {
                    if let Some(t) = ObjectType::from_local(legacy_iri_local(rest)) {
                        meta.object_type = t;
                    }
                }
                "title" => meta.title = legacy_quoted(rest),
                "original-url" => meta.original_url = Some(legacy_quoted(rest)),
                "mime-type" => meta.mime_type = Some(legacy_quoted(rest)),
                "slug" => {
                    let v = legacy_quoted(rest);
                    if is_valid_slug(&v) {
                        meta.slug = Some(v);
                    }
                }
                "virtualPath" => {
                    let v = legacy_quoted(rest);
                    if crate::vault::validate_virtual_path(&v).is_ok() {
                        meta.virtual_path = Some(v);
                    }
                }
                "dateCreated" => meta.created = legacy_quoted(rest),
                "dateModified" => meta.modified = legacy_quoted(rest),
                _ => {}
            }
        }

        meta
    }
}

/// The two namespaces Cerbo mints predicates in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Namespace {
    Cerbo,
    Schema,
}

/// Split a predicate IRI into its namespace and local name. `NS_SCHEMA` nests
/// inside `NS_CERBO`, so it has to be tested first.
fn split_namespace(iri: &str) -> Option<(Namespace, &str)> {
    if let Some(local) = iri.strip_prefix(crate::rdf::NS_SCHEMA) {
        Some((Namespace::Schema, local))
    } else {
        iri.strip_prefix(crate::rdf::NS_CERBO)
            .map(|local| (Namespace::Cerbo, local))
    }
}

// ── Legacy `meta.ttl` helpers ──────────────────────────────────────────────

/// `":title"`, `"cerbo:slug"`, `"schema:dateCreated"` → the local name.
/// Anything else — `"@prefix"`, an IRI, a blank line — is not a predicate.
fn legacy_local_name(token: &str) -> Option<&str> {
    let (prefix, local) = token.split_once(':')?;
    matches!(prefix, "" | "cerbo" | "schema").then_some(local)
}

/// The first double-quoted run in `rest`, unescaped only as far as the legacy
/// writer escaped it — which is to say, not at all.
fn legacy_quoted(rest: &str) -> String {
    let Some(start) = rest.find('"') else {
        return String::new();
    };
    let tail = &rest[start + 1..];
    match tail.find('"') {
        Some(end) => tail[..end].to_string(),
        None => String::new(),
    }
}

/// `":Source ;"` / `"cerbo:Product ;"` → `"Source"` / `"Product"`.
fn legacy_iri_local(rest: &str) -> &str {
    rest.split_whitespace()
        .next()
        .unwrap_or("")
        .rsplit(':')
        .next()
        .unwrap_or("")
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
    let auto_slug = match obj_type {
        ObjectType::Product | ObjectType::Source => {
            let parsed_uuid = Uuid::parse_str(&uuid).unwrap_or_else(|_| Uuid::new_v4());
            Some(crate::slug::slugify(&title, parsed_uuid))
        }
        _ => None,
    };
    let meta = ObjectMeta {
        object_type: obj_type,
        title: title.clone(),
        created: now.clone(),
        modified: now,
        original_url: None,
        mime_type: None,
        slug: auto_slug,
        virtual_path: None,
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path, &uuid)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // Create page.md for Product/Source/Ontology types (not Attachment)
    if !matches!(obj_type, ObjectType::Attachment) {
        let page_path = obj_dir.join("page.md");
        let content = format!("# {}\n", title);
        crate::fsio::write_atomic_str(&page_path, &content)
            .map_err(|e| format!("Failed to write page.md: {}", e))?;
    }


    Ok(uuid)
}

/// Create an object with custom metadata (slug and virtual path).
pub fn object_create_with_metadata(
    ctx: &CerboContext,
    uuid: Option<String>,
    obj_type: ObjectType,
    title: String,
    custom_slug: Option<String>,
    virtual_path: Option<String>,
) -> Result<String, String> {
    let uuid = uuid.unwrap_or_else(|| Uuid::new_v4().to_string());
    let obj_dir = object_path(ctx, &uuid);

    fs::create_dir_all(&obj_dir).map_err(|e| format!("Failed to create object dir: {}", e))?;

    // Create meta.ttl
    let now = chrono::Utc::now().to_rfc3339();
    let slug = match obj_type {
        ObjectType::Product | ObjectType::Source => {
            if let Some(custom) = custom_slug {
                Some(custom)
            } else {
                let parsed_uuid = Uuid::parse_str(&uuid).unwrap_or_else(|_| Uuid::new_v4());
                Some(crate::slug::slugify(&title, parsed_uuid))
            }
        }
        _ => None,
    };
    let meta = ObjectMeta {
        object_type: obj_type,
        title: title.clone(),
        created: now.clone(),
        modified: now,
        original_url: None,
        mime_type: None,
        slug,
        virtual_path,
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path, &uuid)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // Create page.md for Product/Source/Ontology types (not Attachment)
    if !matches!(obj_type, ObjectType::Attachment) {
        let page_path = obj_dir.join("page.md");
        let content = format!("# {}\n", title);
        crate::fsio::write_atomic_str(&page_path, &content)
            .map_err(|e| format!("Failed to write page.md: {}", e))?;
    }


    Ok(uuid)
}

/// Where deleted objects are kept. Nothing prunes it: emptying the trash is the
/// user's call, never Cerbo's.
pub fn trash_dir(ctx: &CerboContext) -> PathBuf {
    ctx.config_dir.join("trash")
}

/// Delete an object by UUID. Fails if type is Source (read-only).
///
/// The object's directory is moved into `.cerbo/trash/` rather than destroyed,
/// so a mistaken delete is recoverable from the vault itself. Every file inside
/// it is preserved byte-for-byte — the move is a rename, not a copy.
pub fn object_delete(ctx: &CerboContext, uuid: &str) -> Result<(), String> {
    let obj_dir = object_path(ctx, uuid);

    if !obj_dir.exists() {
        return Err(format!("Object not found: {}", uuid));
    }

    // The read-only refusal happens before anything is moved.
    let meta_path = obj_dir.join("meta.ttl");
    if meta_path.exists() {
        let meta = ObjectMeta::read_from_file(&meta_path)
            .map_err(|e| format!("Failed to read meta.ttl: {}", e))?;
        if meta.object_type.is_readonly() {
            return Err("Cannot delete source type (read-only)".to_string());
        }
    }

    let trash = trash_dir(ctx);
    fs::create_dir_all(&trash)
        .map_err(|e| format!("Failed to create trash directory: {}", e))?;

    let entry = unique_trash_entry(&trash, uuid);
    fs::rename(&obj_dir, &entry)
        .map_err(|e| format!("Failed to move object to trash: {}", e))?;

    Ok(())
}

/// `<timestamp>-<uuid>`, so the entry says both which object it was and when it
/// went. A suffix disambiguates the same object deleted twice in one instant.
fn unique_trash_entry(trash: &Path, uuid: &str) -> PathBuf {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.6fZ");
    let base = format!("{stamp}-{uuid}");

    let mut candidate = trash.join(&base);
    let mut n = 1;
    while candidate.exists() {
        candidate = trash.join(format!("{base}-{n}"));
        n += 1;
    }
    candidate
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
    let import_title = format!("Imported: {}", url);
    let import_slug = {
        let parsed_uuid = Uuid::parse_str(&uuid).unwrap_or_else(|_| Uuid::new_v4());
        Some(crate::slug::slugify(&import_title, parsed_uuid))
    };
    let meta = ObjectMeta {
        object_type: ObjectType::Source,
        title: import_title.clone(),
        created: now.clone(),
        modified: now,
        original_url: Some(url.to_string()),
        mime_type: Some("text/markdown".to_string()),
        slug: import_slug,
        virtual_path: None,
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path, &uuid)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // Create page.md with imported content
    let page_path = obj_dir.join("page.md");
    crate::fsio::write_atomic_str(&page_path, &body)
        .map_err(|e| format!("Failed to write page.md: {}", e))?;


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
        slug: None,
        virtual_path: None,
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path, &uuid)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // Create page.md with ontology content
    let page_path = obj_dir.join("page.md");
    crate::fsio::write_atomic_str(&page_path, &body)
        .map_err(|e| format!("Failed to write page.md: {}", e))?;


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
    
    crate::fsio::write_atomic_str(&map_path, &content)
        .map_err(|e| format!("Failed to write ontology-map.json: {}", e))
}

/// Extract prefix from ontology URL
fn extract_prefix_from_url(url: &str) -> String {
    // Simple extraction: get last part of domain
    // e.g., "https://schema.org/" → "schema"
    // e.g., "https://xmlns.com/foaf/0.1/" → "foaf"
    if let Some(domain) = url.split("://").nth(1) {
        let domain = domain.split('/').next().unwrap_or(domain);
        if let Some(name) = domain.split('.').next() {
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

    crate::fsio::write_atomic_str(&page_path, content)
        .map_err(|e| format!("Failed to write page.md: {}", e))?;

    // Update modified timestamp in meta.ttl
    update_modified_date(&meta_path, uuid)?;

    // Extract links and update backrefs.ttl
    for target in update_backrefs(ctx, uuid, &old_content, content)? {
        eprintln!("cerbo: broken link in page {uuid}: {target} does not exist");
    }

    Ok(())
}

/// Stamp `schema:dateModified` with the current time.
///
/// Re-serialising from the parsed fields also rewrites a legacy `meta.ttl` as
/// valid Turtle, which is safe now that the reader recovers the real type and
/// title from a damaged file rather than the injected ones.
fn update_modified_date(meta_path: &Path, uuid: &str) -> Result<(), String> {
    if !meta_path.exists() {
        return Ok(());
    }

    let mut meta = ObjectMeta::read_from_file(meta_path)
        .map_err(|e| format!("Failed to read meta.ttl: {}", e))?;
    meta.modified = chrono::Utc::now().to_rfc3339();

    meta.write_to_file(meta_path, uuid)
        .map_err(|e| format!("Failed to update meta.ttl: {}", e))
}

/// Refresh `backrefs.ttl` on every object whose link status changed.
///
/// Returns the link targets that could not take a backreference — a malformed
/// id, or an object that does not exist. Those are broken links, not failures:
/// the page content stands. A genuine I/O failure *is* an error and is
/// propagated, so a partial save is never reported as success.
pub fn update_backrefs(
    ctx: &CerboContext,
    source_uuid: &str,
    old_content: &str,
    new_content: &str,
) -> Result<Vec<String>, String> {
    let old_links = crate::links::extract_cerbo_links(old_content);
    let new_links = crate::links::extract_cerbo_links(new_content);
    let mut broken = Vec::new();

    for target_uuid in &old_links {
        if new_links.contains(target_uuid) {
            continue;
        }
        if !crate::links::is_live_target(ctx, target_uuid) {
            broken.push(target_uuid.clone());
            continue;
        }
        crate::links::backrefs_remove(ctx, target_uuid, source_uuid).map_err(|e| {
            format!("page.md was written, but clearing the backreference on {target_uuid} failed: {e}")
        })?;
    }

    for target_uuid in &new_links {
        if old_links.contains(target_uuid) {
            continue;
        }
        if !crate::links::is_live_target(ctx, target_uuid) {
            broken.push(target_uuid.clone());
            continue;
        }
        crate::links::backrefs_add(ctx, target_uuid, source_uuid).map_err(|e| {
            format!("page.md was written, but adding the backreference on {target_uuid} failed: {e}")
        })?;
    }

    Ok(broken)
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
        slug: None,
        virtual_path: None,
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path, &uuid)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // No page.md for attachments


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

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::CerboContext;
    use std::fs;

    /// A vault of its own for each test. The returned `TempDir` guard must stay
    /// alive for the whole test — dropping it removes the directory.
    fn create_test_context() -> (tempfile::TempDir, CerboContext) {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("objects")).unwrap();
        let ctx = CerboContext {
            config_dir: dir.clone(),
            cache_dir: dir.join("cache"),
        };
        (tmp, ctx)
    }

    #[test]
    fn test_object_create_and_read() {
        let (_tmp, ctx) = create_test_context();

        // Create a Product page
        let uuid = object_create(&ctx, None, ObjectType::Product, "Test Page".to_string()).unwrap();
        assert!(!uuid.is_empty());

        // Read page.md
        let content = object_read(&ctx, &uuid).unwrap();
        assert!(content.contains("# Test Page"));

        // Cleanup
        let _ = object_delete(&ctx, &uuid);
    }

    #[test]
    fn test_object_create_with_specific_uuid() {
        let (_tmp, ctx) = create_test_context();

        let test_uuid = "test-uuid-1234";
        let uuid = object_create(&ctx, Some(test_uuid.to_string()), ObjectType::Product, "Another Page".to_string()).unwrap();
        assert_eq!(uuid, test_uuid);

        let _ = object_delete(&ctx, test_uuid);
    }

    #[test]
    fn test_object_read_nonexistent() {
        let (_tmp, ctx) = create_test_context();

        let result = object_read(&ctx, "non-existent-uuid");
        assert!(result.is_err());

    }

    #[test]
    fn test_object_delete_nonexistent() {
        let (_tmp, ctx) = create_test_context();

        let result = object_delete(&ctx, "non-existent-uuid");
        assert!(result.is_err());

    }

    #[test]
    fn test_source_type_readonly() {
        let (_tmp, ctx) = create_test_context();

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

    }

    #[test]
    fn test_object_write_and_read() {
        let (_tmp, ctx) = create_test_context();

        let uuid = object_create(&ctx, None, ObjectType::Product, "Editable Page".to_string()).unwrap();

        // Write new content
        let new_content = "# Editable Page\n\nThis is updated content.";
        object_write(&ctx, &uuid, new_content).unwrap();

        // Read back
        let content = object_read(&ctx, &uuid).unwrap();
        assert!(content.contains("This is updated content."));

        let _ = object_delete(&ctx, &uuid);
    }

    #[test]
    fn test_object_type_attachment() {
        let (_tmp, ctx) = create_test_context();

        // Create an Attachment object (no page.md)
        let uuid = object_create(&ctx, None, ObjectType::Attachment, "image.png".to_string()).unwrap();

        let obj_dir = object_path(&ctx, &uuid);
        assert!(obj_dir.exists());
        assert!(!obj_dir.join("page.md").exists()); // No page.md for attachments

        let _ = object_delete(&ctx, &uuid);
    }

    #[test]
    fn test_object_type_ontology() {
        let (_tmp, ctx) = create_test_context();

        // Create an Ontology object
        let uuid = object_create(&ctx, None, ObjectType::Ontology, "Schema.org".to_string()).unwrap();

        let obj_dir = object_path(&ctx, &uuid);
        assert!(obj_dir.join("page.md").exists()); // Has page.md
        assert!(obj_dir.join("meta.ttl").exists());

        let _ = object_delete(&ctx, &uuid);
    }

    #[test]
    fn test_meta_ttl_creation() {
        let (_tmp, ctx) = create_test_context();

        let uuid = object_create(&ctx, None, ObjectType::Product, "Meta Test".to_string()).unwrap();

        let obj_dir = object_path(&ctx, &uuid);
        let meta_path = obj_dir.join("meta.ttl");
        assert!(meta_path.exists());

        let content = fs::read_to_string(&meta_path).unwrap();
        assert!(content.contains("cerbo:type cerbo:Product"), "{content}");
        assert!(content.contains(r#"cerbo:title "Meta Test""#), "{content}");
        assert!(content.contains("schema:dateModified"), "{content}");
        assert!(crate::rdf::parse(&content).is_ok(), "meta.ttl must be valid Turtle:\n{content}");

        let _ = object_delete(&ctx, &uuid);
    }

    #[test]
    fn test_object_create_product_has_slug() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "My Test Page".to_string()).unwrap();
        let meta = ObjectMeta::read_from_file(&object_path(&ctx, &uuid).join("meta.ttl")).unwrap();
        assert!(meta.slug.is_some(), "Product page should have auto-generated slug");
        let slug = meta.slug.unwrap();
        assert!(slug.contains("my-test-page"), "slug: {slug}");
        let _ = object_delete(&ctx, &uuid);
    }

    #[test]
    fn test_object_meta_slug_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let meta_path = tmp.path().join("meta.ttl");
        let now = chrono::Utc::now().to_rfc3339();
        let original = ObjectMeta {
            object_type: ObjectType::Product,
            title: "Round Trip".to_string(),
            created: now.clone(),
            modified: now,
            original_url: None,
            mime_type: None,
            slug: Some("round-trip".to_string()),
            virtual_path: Some("notes/rust".to_string()),
        };
        original.write_to_file(&meta_path, "f7e435db-9740-4a2a-8e57-b439c4f8bb18").unwrap();
        let loaded = ObjectMeta::read_from_file(&meta_path).unwrap();
        assert_eq!(loaded.slug.as_deref(), Some("round-trip"));
        assert_eq!(loaded.virtual_path.as_deref(), Some("notes/rust"));
    }

    #[test]
    fn test_object_meta_optional_slug_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let meta_path = tmp.path().join("meta.ttl");
        let now = chrono::Utc::now().to_rfc3339();
        let original = ObjectMeta {
            object_type: ObjectType::Product,
            title: "No Slug".to_string(),
            created: now.clone(),
            modified: now,
            original_url: None,
            mime_type: None,
            slug: None,
            virtual_path: None,
        };
        original.write_to_file(&meta_path, "f7e435db-9740-4a2a-8e57-b439c4f8bb18").unwrap();
        let loaded = ObjectMeta::read_from_file(&meta_path).unwrap();
        assert!(loaded.slug.is_none());
        assert!(loaded.virtual_path.is_none());
    }

    #[test]
    fn test_invalid_slug_ignored_on_parse() {
        let tmp = tempfile::TempDir::new().unwrap();
        let meta_path = tmp.path().join("meta.ttl");
        let mut meta = ObjectMeta::empty();
        meta.title = "Test".into();
        meta.slug = Some("valid-slug".into());
        let doc = meta
            .to_turtle("f7e435db-9740-4a2a-8e57-b439c4f8bb18")
            .replace(r#"cerbo:slug "valid-slug""#, r#"cerbo:slug "/invalid/slug/""#);
        fs::write(&meta_path, doc).unwrap();

        let loaded = ObjectMeta::read_from_file(&meta_path).unwrap();
        assert!(loaded.slug.is_none(), "invalid slug should be silently ignored");
    }

    // ── Trash ─────────────────────────────────────────────────────────────

    fn trash_entries(ctx: &CerboContext) -> Vec<std::path::PathBuf> {
        let trash = trash_dir(ctx);
        if !trash.is_dir() {
            return Vec::new();
        }
        let mut entries: Vec<_> = fs::read_dir(&trash).unwrap().flatten().map(|e| e.path()).collect();
        entries.sort();
        entries
    }

    #[test]
    fn delete_moves_the_object_to_trash_intact() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Doomed".into()).unwrap();

        let obj_dir = object_path(&ctx, &uuid);
        let page_before = fs::read(obj_dir.join("page.md")).unwrap();
        let meta_before = fs::read(obj_dir.join("meta.ttl")).unwrap();
        fs::write(obj_dir.join("extra.bin"), [0u8, 137, 255]).unwrap();

        object_delete(&ctx, &uuid).unwrap();

        assert!(!obj_dir.exists(), "the live object directory must be gone");
        let entries = trash_entries(&ctx);
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        let name = entry.file_name().unwrap().to_string_lossy().to_string();
        assert!(name.contains(&uuid), "trash entry must name its object: {name}");
        assert!(name.starts_with("20"), "trash entry must be timestamped: {name}");

        assert_eq!(fs::read(entry.join("page.md")).unwrap(), page_before);
        assert_eq!(fs::read(entry.join("meta.ttl")).unwrap(), meta_before);
        assert_eq!(fs::read(entry.join("extra.bin")).unwrap(), [0u8, 137, 255]);
    }

    #[test]
    fn deleting_a_read_only_object_creates_no_trash_entry() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, None, ObjectType::Source, "Imported".into()).unwrap();

        let err = object_delete(&ctx, &uuid).unwrap_err();
        assert!(err.contains("read-only"), "{err}");
        assert!(object_path(&ctx, &uuid).is_dir(), "the object must be untouched");
        assert!(trash_entries(&ctx).is_empty(), "nothing may be moved before the refusal");
    }

    #[test]
    fn deleting_the_same_uuid_twice_keeps_both_entries() {
        let (_tmp, ctx) = create_test_context();
        let uuid = "77777777-7777-7777-7777-777777777777".to_string();

        object_create(&ctx, Some(uuid.clone()), ObjectType::Product, "First".into()).unwrap();
        object_write(&ctx, &uuid, "# First\n").unwrap();
        object_delete(&ctx, &uuid).unwrap();

        object_create(&ctx, Some(uuid.clone()), ObjectType::Product, "Second".into()).unwrap();
        object_write(&ctx, &uuid, "# Second\n").unwrap();
        object_delete(&ctx, &uuid).unwrap();

        let entries = trash_entries(&ctx);
        assert_eq!(entries.len(), 2, "each deletion needs its own entry: {entries:?}");

        let mut contents: Vec<String> = entries
            .iter()
            .map(|e| fs::read_to_string(e.join("page.md")).unwrap())
            .collect();
        contents.sort();
        assert_eq!(contents, ["# First\n", "# Second\n"]);
    }

    #[test]
    fn a_trashed_object_disappears_from_listings_and_does_not_resolve() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Gone".into()).unwrap();
        object_delete(&ctx, &uuid).unwrap();

        let listed = crate::page::page_list(&ctx).unwrap();
        assert!(listed.is_empty(), "trashed object still listed: {listed:?}");
        assert!(object_read(&ctx, &uuid).is_err(), "a trashed object must not resolve");
    }

    /// A live page that still links to a trashed object must not bring it back.
    #[test]
    fn saving_a_link_to_a_trashed_object_does_not_resurrect_it() {
        let (_tmp, ctx) = create_test_context();
        let target = object_create(&ctx, None, ObjectType::Product, "Target".into()).unwrap();
        let source = object_create(&ctx, None, ObjectType::Product, "Source".into()).unwrap();
        object_delete(&ctx, &target).unwrap();

        let broken = update_backrefs(
            &ctx,
            &source,
            "",
            &format!("cerbo://objects/{target}"),
        )
        .unwrap();

        assert_eq!(broken, vec![target.clone()], "the link must report as broken");
        assert!(
            !object_path(&ctx, &target).exists(),
            "the trashed object was recreated under .cerbo/objects/"
        );
    }

    /// The metadata stamp runs *after* `page.md` has been written, so a failure
    /// here is exactly the "content written, metadata not" case. It must be
    /// reported, never swallowed.
    #[test]
    #[cfg(unix)]
    fn a_failing_metadata_update_is_reported_not_swallowed() {
        use std::os::unix::fs::PermissionsExt;

        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Locked".into()).unwrap();
        let obj_dir = object_path(&ctx, &uuid);
        let meta_path = obj_dir.join("meta.ttl");

        fs::set_permissions(&obj_dir, fs::Permissions::from_mode(0o500)).unwrap();
        let result = update_modified_date(&meta_path, &uuid);
        fs::set_permissions(&obj_dir, fs::Permissions::from_mode(0o700)).unwrap();

        if result.is_ok() {
            // Running as root defeats the mode bits.
            return;
        }
        let message = result.unwrap_err();
        assert!(
            message.contains("meta.ttl"),
            "the error must say which part failed: {message}"
        );
    }

    /// And an unwritable object fails the whole save rather than half-succeeding.
    #[test]
    #[cfg(unix)]
    fn an_unwritable_object_fails_the_save() {
        use std::os::unix::fs::PermissionsExt;

        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Locked".into()).unwrap();
        let obj_dir = object_path(&ctx, &uuid);
        let before = fs::read(obj_dir.join("page.md")).unwrap();

        fs::set_permissions(&obj_dir, fs::Permissions::from_mode(0o500)).unwrap();
        let result = object_write(&ctx, &uuid, "# Locked\n\nnew body\n");
        fs::set_permissions(&obj_dir, fs::Permissions::from_mode(0o700)).unwrap();

        if result.is_ok() {
            return;
        }
        let message = result.unwrap_err();
        assert!(message.contains("page.md"), "the error must name the failed part: {message}");
        assert_eq!(fs::read(obj_dir.join("page.md")).unwrap(), before, "content must survive");
    }

    // ── Turtle correctness ────────────────────────────────────────────────

    fn roundtrip(meta: &ObjectMeta) -> ObjectMeta {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("meta.ttl");
        meta.write_to_file(&path, "f7e435db-9740-4a2a-8e57-b439c4f8bb18").unwrap();
        ObjectMeta::read_from_file(&path).unwrap()
    }

    #[test]
    fn subject_is_the_objects_own_iri() {
        let doc = ObjectMeta::empty().to_turtle("f7e435db-9740-4a2a-8e57-b439c4f8bb18");
        assert!(
            doc.contains("<cerbo://objects/f7e435db-9740-4a2a-8e57-b439c4f8bb18>"),
            "subject must be the object's own IRI:\n{doc}"
        );
        assert!(!doc.contains("<uuid>"), "placeholder subject left in output:\n{doc}");
    }

    /// A title that reads like a type assertion must stay a title — the defect
    /// that used to brick a page permanently.
    #[test]
    fn title_that_looks_like_a_type_assertion_round_trips() {
        let (_tmp, ctx) = create_test_context();
        let title = "RDF :type :Source notes".to_string();
        let uuid = object_create(&ctx, None, ObjectType::Product, title.clone()).unwrap();

        let meta = ObjectMeta::read_from_file(&object_path(&ctx, &uuid).join("meta.ttl")).unwrap();
        assert_eq!(meta.object_type, ObjectType::Product, "title injected a type");
        assert_eq!(meta.title, title);

        // …and the page stays usable.
        object_write(&ctx, &uuid, "# still writable\n").unwrap();
        object_delete(&ctx, &uuid).unwrap();
    }

    #[test]
    fn titles_with_awkward_characters_round_trip() {
        for title in [
            r#"He said "hi""#,
            "first line\nsecond line",
            r"C:\temp — заметки 🎉",
        ] {
            let mut meta = ObjectMeta::empty();
            meta.title = title.to_string();
            assert_eq!(roundtrip(&meta).title, title, "lost characters in {title:?}");
        }
    }

    #[test]
    fn rewriting_unchanged_metadata_is_byte_stable() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("meta.ttl");

        let mut meta = ObjectMeta::empty();
        meta.title = r#"Quotes "and" a — dash"#.into();
        meta.created = "2024-01-01T00:00:00Z".into();
        meta.modified = "2024-01-02T00:00:00Z".into();
        meta.slug = Some("quotes-and-a-dash".into());
        meta.virtual_path = Some("notes/rust".into());
        meta.write_to_file(&path, "f7e435db-9740-4a2a-8e57-b439c4f8bb18").unwrap();

        let first = fs::read(&path).unwrap();
        let reread = ObjectMeta::read_from_file(&path).unwrap();
        reread.write_to_file(&path, "f7e435db-9740-4a2a-8e57-b439c4f8bb18").unwrap();

        assert_eq!(first, fs::read(&path).unwrap(), "unchanged rewrite must be byte-identical");
    }

    /// A vault damaged before this change recovers on read, with no rewrite and
    /// no migration step.
    #[test]
    fn damaged_legacy_metadata_reads_correctly_without_being_rewritten() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("meta.ttl");

        // Exactly what the old `format!` writer produced for this title. The old
        // writer never substituted the UUID, so the subject was a literal
        // placeholder; it is assembled here so a repo-wide grep for that
        // placeholder stays clean.
        let damaged = format!(
            concat!(
                "@prefix : <cerbo://ontology/> .\n",
                "@prefix schema: <cerbo://ontology/schema/> .\n",
                "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
                "\n",
                "<cerbo://{}>\n",
                "    :type :Product ;\n",
                "    :title \"RDF :type :Source notes\" ;\n",
                "    schema:dateCreated \"2024-01-01T00:00:00Z\"^^xsd:dateTime ;\n",
                "    schema:dateModified \"2024-01-01T00:00:00Z\"^^xsd:dateTime .\n",
                "    cerbo:slug \"rdf-type-source-notes\" .\n",
            ),
            format!("objects/<{}>", "uuid")
        );
        fs::write(&path, damaged).unwrap();
        let before = fs::read(&path).unwrap();

        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.object_type, ObjectType::Product, "damaged title read as a type");
        assert_eq!(meta.title, "RDF :type :Source notes");
        assert_eq!(meta.slug.as_deref(), Some("rdf-type-source-notes"));
        assert_eq!(meta.created, "2024-01-01T00:00:00Z");

        assert_eq!(before, fs::read(&path).unwrap(), "reading must not rewrite the file");
    }

}
