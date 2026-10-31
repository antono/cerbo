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

    /// The inverse of [`ObjectType::as_str`]: the local name of a legacy `cerbo:`
    /// class, as found in `meta.ttl` written before the schema.org switch.
    pub fn from_local(local: &str) -> Option<Self> {
        match local {
            "Product" => Some(ObjectType::Product),
            "Source" => Some(ObjectType::Source),
            "Attachment" => Some(ObjectType::Attachment),
            "Ontology" => Some(ObjectType::Ontology),
            _ => None,
        }
    }

    /// The schema.org class recorded as this object's `rdf:type`.
    pub fn class_iri(&self) -> oxrdf::NamedNode {
        crate::rdf::schema(match self {
            ObjectType::Product => "NoteDigitalDocument",
            ObjectType::Source => "WebPage",
            ObjectType::Attachment => "MediaObject",
            ObjectType::Ontology => "DefinedTermSet",
        })
    }

    /// The inverse of [`ObjectType::class_iri`]. Any other class — a user's own
    /// `schema:Recipe`, say — is not an object class and yields `None`.
    pub fn from_class_iri(iri: &str) -> Option<Self> {
        [
            ObjectType::Product,
            ObjectType::Source,
            ObjectType::Attachment,
            ObjectType::Ontology,
        ]
        .into_iter()
        .find(|t| t.class_iri().as_str() == iri)
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
    ///
    /// Triples in the file on disk that Cerbo does not model — a user's own
    /// `rdf:type schema:Recipe`, a statement about a linked resource — are
    /// carried over, so a save never deletes what a hand edit or the vocabulary
    /// migration put there.
    pub fn write_to_file(&self, meta_path: &Path, uuid: &str) -> io::Result<()> {
        let extra = fs::read_to_string(meta_path)
            .ok()
            .and_then(|content| crate::rdf::parse(&content).ok())
            .map(|triples| Self::unmodelled_triples(&triples, Some(uuid)))
            .unwrap_or_default();
        crate::fsio::write_atomic_str(meta_path, &self.to_turtle_with(uuid, &extra))
    }

    /// Read `meta.ttl` from Turtle.
    ///
    /// A file that records more than one object class is refused with
    /// `InvalidData`: guessing could make a note read-only or vice versa.
    pub fn read_from_file(meta_path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(meta_path)?;
        // The directory is the object's UUID; it names the object in errors
        // when the file's own subject does not.
        let dir_uuid = meta_path
            .parent()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned());
        Self::from_turtle(&content, dir_uuid.as_deref())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Serialise as Turtle, with the object's own IRI as the subject.
    pub fn to_turtle(&self, uuid: &str) -> String {
        self.to_turtle_with(uuid, &[])
    }

    /// [`Self::to_turtle`], followed by `extra` — triples Cerbo does not model,
    /// as [`Self::unmodelled_triples`] returns them — in their original order,
    /// each written once.
    pub(crate) fn to_turtle_with(&self, uuid: &str, extra: &[oxrdf::Triple]) -> String {
        use crate::rdf;
        use oxrdf::{Literal, NamedNode, Term, Triple};

        let subject = rdf::object_iri(uuid);
        let text = |v: &str| Term::from(Literal::new_simple_literal(v));

        let mut triples = vec![
            Triple::new(
                subject.clone(),
                rdf::rdf_type(),
                Term::from(self.object_type.class_iri()),
            ),
            Triple::new(subject.clone(), rdf::schema("name"), text(&self.title)),
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
        if let Some(url) = self.original_url.as_deref() {
            // `schema:url` is an IRI. A URL that is not a valid IRI is still the
            // user's source, so it is kept as a literal rather than dropped.
            let object = NamedNode::new(url).map(Term::from).unwrap_or_else(|_| text(url));
            triples.push(Triple::new(subject.clone(), rdf::schema("url"), object));
        }
        for (predicate, value) in [
            (rdf::schema("encodingFormat"), self.mime_type.as_deref()),
            (rdf::cerbo("slug"), self.slug.as_deref()),
            (rdf::cerbo("virtualPath"), self.virtual_path.as_deref()),
        ] {
            if let Some(v) = value {
                triples.push(Triple::new(subject.clone(), predicate, text(v)));
            }
        }

        // A file holding both the legacy and the standard spelling of a fact
        // upgrades into the same triple twice; it is written once.
        let mut seen = std::collections::HashSet::new();
        triples.extend(extra.iter().filter(|t| seen.insert(*t)).cloned());

        rdf::serialize(&triples)
    }

    /// The triples of a parsed `meta.ttl` that [`Self::from_triples`] does not
    /// turn into a field: anything about another subject, and, about the object
    /// itself, every predicate other than its object class, `schema:name`, the
    /// dates, `schema:url`, `schema:encodingFormat`, `cerbo:slug` and
    /// `cerbo:virtualPath`. Writers carry these over unchanged.
    pub(crate) fn unmodelled_triples(
        triples: &[oxrdf::Triple],
        dir_uuid: Option<&str>,
    ) -> Vec<oxrdf::Triple> {
        let own = own_subject(triples, dir_uuid);
        triples
            .iter()
            .filter(|t| own.as_ref() != Some(&t.subject) || !is_modelled(t))
            .cloned()
            .collect()
    }

    fn from_turtle(content: &str, dir_uuid: Option<&str>) -> Result<Self, String> {
        match crate::rdf::parse(content) {
            Ok(triples) => Self::from_triples(&triples, dir_uuid),
            // Vaults written before this change hold `meta.ttl` that is not valid
            // Turtle: `cerbo:` was used without being declared, and the optional
            // predicates dangle after a statement terminator. Those files must keep
            // opening; they are rewritten valid on the object's next save.
            Err(_) => Self::from_legacy_lines(content, dir_uuid),
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
    ///
    /// [`crate::rdf::parse`] has already rewritten legacy spellings, so only the
    /// standard vocabulary is matched here.
    ///
    /// Only statements about the object itself are read (see [`own_subject`]):
    /// a user's triple about a linked resource — `<https://example.org/137> a
    /// schema:WebPage` — neither changes the object's class nor its title.
    pub(crate) fn from_triples(triples: &[oxrdf::Triple], dir_uuid: Option<&str>) -> Result<Self, String> {
        use crate::rdf::{NS_CERBO, NS_SCHEMA};
        use oxrdf::{NamedOrBlankNode, Term};

        let mut meta = Self::empty();
        let mut classes: Vec<ObjectType> = Vec::new();
        let own = own_subject(triples, dir_uuid);
        let subject_uuid = match &own {
            Some(NamedOrBlankNode::NamedNode(n)) => crate::rdf::uuid_from_object_iri(n.as_str()),
            _ => None,
        };

        for triple in triples.iter().filter(|t| own.as_ref() == Some(&t.subject)) {
            let predicate = triple.predicate.as_str();
            let literal = match &triple.object {
                Term::Literal(l) => Some(l.value()),
                _ => None,
            };

            if triple.predicate == oxrdf::vocab::rdf::TYPE {
                if let Term::NamedNode(n) = &triple.object
                    && let Some(t) = ObjectType::from_class_iri(n.as_str())
                    && !classes.contains(&t)
                {
                    classes.push(t);
                }
                continue;
            }

            if let Some(local) = predicate.strip_prefix(NS_SCHEMA) {
                match local {
                    "name" => {
                        if let Some(v) = literal {
                            meta.title = v.to_string();
                        }
                    }
                    // An IRI since the schema.org switch; a literal before it.
                    "url" => {
                        meta.original_url = match &triple.object {
                            Term::NamedNode(n) => Some(n.as_str().to_string()),
                            _ => literal.map(str::to_string),
                        };
                    }
                    "encodingFormat" => meta.mime_type = literal.map(str::to_string),
                    "dateCreated" => {
                        if let Some(v) = literal {
                            meta.created = v.to_string();
                        }
                    }
                    "dateModified" => {
                        if let Some(v) = literal {
                            meta.modified = v.to_string();
                        }
                    }
                    _ => {}
                }
            } else if let Some(local) = predicate.strip_prefix(NS_CERBO) {
                match local {
                    "slug" => {
                        meta.slug = literal.filter(|v| is_valid_slug(v)).map(str::to_string);
                    }
                    "virtualPath" => {
                        meta.virtual_path = literal
                            .filter(|v| crate::vault::validate_virtual_path(v).is_ok())
                            .map(str::to_string);
                    }
                    _ => {}
                }
            }
        }

        meta.object_type = single_class(&classes, subject_uuid.or(dir_uuid))?;
        Ok(meta)
    }

    /// [`Self::from_legacy_lines`] for callers that must not mistake an arbitrary
    /// broken file for an empty note: `None` unless a line names one of the four
    /// legacy object classes, as every file the old writer produced did.
    pub(crate) fn from_legacy_lines_checked(
        content: &str,
        dir_uuid: Option<&str>,
    ) -> Option<Result<Self, String>> {
        let names_a_class = content.lines().any(|line| {
            line.trim()
                .split_once(char::is_whitespace)
                .is_some_and(|(predicate, rest)| {
                    legacy_local_name(predicate) == Some("type")
                        && ObjectType::from_local(legacy_iri_local(rest)).is_some()
                })
        });
        names_a_class.then(|| Self::from_legacy_lines(content, dir_uuid))
    }

    /// Reader for `meta.ttl` written before this change, which is not valid Turtle.
    ///
    /// The predicate is matched at the start of the line, never anywhere inside it,
    /// so a title like `RDF :type :Source notes` stays a title. Two different
    /// classes are refused, exactly as in a Turtle file.
    fn from_legacy_lines(content: &str, dir_uuid: Option<&str>) -> Result<Self, String> {
        let mut meta = Self::empty();
        let mut classes: Vec<ObjectType> = Vec::new();

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
                    if let Some(t) = ObjectType::from_local(legacy_iri_local(rest))
                        && !classes.contains(&t)
                    {
                        classes.push(t);
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

        meta.object_type = single_class(&classes, dir_uuid)?;
        Ok(meta)
    }
}

/// The object's class from the classes a `meta.ttl` names: none means a note,
/// as it always has; two or more is refused with an error naming the UUID and
/// every class, because guessing could make a note read-only or vice versa.
fn single_class(classes: &[ObjectType], uuid: Option<&str>) -> Result<ObjectType, String> {
    match classes {
        [] => Ok(ObjectType::Product),
        [only] => Ok(*only),
        several => {
            let names: Vec<String> = several
                .iter()
                .map(|t| format!("<{}>", t.class_iri().as_str()))
                .collect();
            Err(format!(
                "object {} has more than one object class in meta.ttl: {}",
                uuid.unwrap_or("<unknown>"),
                names.join(", ")
            ))
        }
    }
}

/// The subject a parsed `meta.ttl` describes the object under: the IRI of the
/// directory's UUID when the file uses it, otherwise the first subject in the
/// object store (a copied object keeps its old IRI until its next write).
fn own_subject(triples: &[oxrdf::Triple], dir_uuid: Option<&str>) -> Option<oxrdf::NamedOrBlankNode> {
    use oxrdf::NamedOrBlankNode;

    if let Some(uuid) = dir_uuid {
        let iri = NamedOrBlankNode::from(crate::rdf::object_iri(uuid));
        if triples.iter().any(|t| t.subject == iri) {
            return Some(iri);
        }
    }
    triples
        .iter()
        .find(|t| match &t.subject {
            NamedOrBlankNode::NamedNode(n) => crate::rdf::uuid_from_object_iri(n.as_str()).is_some(),
            _ => false,
        })
        .map(|t| t.subject.clone())
}

/// Whether [`ObjectMeta::from_triples`] reads this triple, about the object
/// itself, into a field — and [`ObjectMeta::to_turtle`] therefore rewrites it.
fn is_modelled(triple: &oxrdf::Triple) -> bool {
    use crate::rdf::{NS_CERBO, NS_SCHEMA};

    if triple.predicate == oxrdf::vocab::rdf::TYPE {
        return matches!(&triple.object, oxrdf::Term::NamedNode(n)
            if ObjectType::from_class_iri(n.as_str()).is_some());
    }
    let predicate = triple.predicate.as_str();
    match (predicate.strip_prefix(NS_SCHEMA), predicate.strip_prefix(NS_CERBO)) {
        (Some(local), _) => matches!(
            local,
            "name" | "dateCreated" | "dateModified" | "url" | "encodingFormat"
        ),
        (_, Some(local)) => matches!(local, "slug" | "virtualPath"),
        _ => false,
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

    sync_index(ctx, &uuid);
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

    sync_index(ctx, &uuid);
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

    sync_index(ctx, uuid);
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
/// Fetches content from URL and creates a Source object (`rdf:type schema:WebPage`)
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

    sync_index(ctx, &uuid);
    Ok(uuid)
}

/// Import an ontology URL as an Ontology object
/// Fetches content and creates an Ontology object (`rdf:type schema:DefinedTermSet`)
///
/// With `registration = Some((prefix, namespace))`, the vault's
/// `ontology-map.json` (in `ctx.config_dir`) then maps `prefix` to `namespace`
/// and the new object's UUID. Without it the map is left alone. The prefix and
/// namespace are checked before anything is fetched or created.
pub fn object_import_ontology(
    ctx: &CerboContext,
    url: &str,
    registration: Option<(&str, &str)>,
) -> Result<String, String> {
    if let Some((prefix, namespace)) = registration {
        crate::prefix_map::validate_registration(prefix, namespace)?;
    }

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

    if let Some((prefix, namespace)) = registration {
        crate::prefix_map::register_prefix(&ctx.config_dir, prefix, namespace, Some(&uuid))?;
    }

    sync_index(ctx, &uuid);
    Ok(uuid)
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
///
/// Only the object's own files are written; the objects it links to are never
/// touched. Its graph in the semantic index is updated afterwards.
pub fn object_write(ctx: &CerboContext, uuid: &str, content: &str) -> Result<(), String> {
    write_page_content(ctx, uuid, content)?;
    sync_index(ctx, uuid);
    Ok(())
}

/// [`object_write`] without the index update, for callers that write more of
/// the object's files first and then call [`sync_index`] once.
pub(crate) fn write_page_content(ctx: &CerboContext, uuid: &str, content: &str) -> Result<(), String> {
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
    crate::fsio::write_atomic_str(&page_path, content)
        .map_err(|e| format!("Failed to write page.md: {}", e))?;

    // Update modified timestamp in meta.ttl
    update_modified_date(&meta_path, uuid)?;

    for target in broken_links(ctx, content) {
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

/// The `cerbo://` link targets in `content` that are not live objects — a
/// malformed id, or an object that does not exist — each once. They are
/// reported, never fixed: nothing is written, and no directory is created.
pub fn broken_links(ctx: &CerboContext, content: &str) -> Vec<String> {
    let mut broken = Vec::new();
    for target in crate::links::extract_cerbo_links(content) {
        if !crate::links::is_live_target(ctx, &target) && !broken.contains(&target) {
            broken.push(target);
        }
    }
    broken
}

// ── Semantic index sync ──────────────────────────────────────

/// The vault that `ctx` writes objects into: its `config_dir` is the vault's
/// `.cerbo/`. Any other context (the global XDG config, a bare test directory)
/// has no semantic index.
fn vault_of(ctx: &CerboContext) -> Option<crate::VaultContext> {
    if ctx.config_dir.file_name()? != ".cerbo" {
        return None;
    }
    Some(crate::VaultContext {
        vault_path: ctx.config_dir.parent()?.to_path_buf(),
        // The index reads only `cache_dir` from the global context.
        global: ctx.clone(),
    })
}

/// Bring the object's graph in the vault's semantic index up to date after its
/// files were written, created or trashed (design D10). Never fails the write;
/// see [`crate::semantic::sync_objects`].
pub(crate) fn sync_index(ctx: &CerboContext, uuid: &str) {
    if let Some(vault_ctx) = vault_of(ctx) {
        crate::semantic::sync_objects(&vault_ctx, &[uuid.to_string()]);
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
        slug: None,
        virtual_path: None,
    };

    let meta_path = obj_dir.join("meta.ttl");
    meta.write_to_file(&meta_path, &uuid)
        .map_err(|e| format!("Failed to write meta.ttl: {}", e))?;

    // No page.md for attachments

    sync_index(ctx, &uuid);
    Ok(uuid)
}

/// Delete an attachment object.
pub fn attachment_delete(ctx: &CerboContext, attachment_uuid: &str) -> Result<(), String> {
    object_delete(ctx, attachment_uuid)
}

/// List the attachments a page uses: the `cerbo://` links in its `page.md`
/// whose target is an Attachment, each once, in order of first appearance.
/// Read from the page itself, so it needs no index.
pub fn attachment_list(ctx: &CerboContext, page_uuid: &str) -> Result<Vec<String>, String> {
    let page_path = object_path(ctx, page_uuid).join("page.md");
    let content = match fs::read_to_string(&page_path) {
        Ok(content) => content,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Failed to read page.md: {}", e)),
    };

    let mut attachments: Vec<String> = Vec::new();
    for target in crate::links::extract_cerbo_links(&content) {
        if attachments.contains(&target) || !crate::links::is_live_target(ctx, &target) {
            continue;
        }
        let meta_path = object_path(ctx, &target).join("meta.ttl");
        match ObjectMeta::read_from_file(&meta_path) {
            Ok(meta) if meta.object_type == ObjectType::Attachment => attachments.push(target),
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => eprintln!("Warning: skipping linked object {target}: {e}"),
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
        assert!(content.contains("schema:NoteDigitalDocument"), "{content}");
        assert!(content.contains(r#"schema:name "Meta Test""#), "{content}");
        assert!(content.contains("schema:dateModified"), "{content}");
        assert!(content.contains("@prefix schema: <https://schema.org/>"), "{content}");
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

    #[test]
    fn attachment_list_reads_the_pages_links() {
        let (tmp, ctx) = create_test_context();
        let file = tmp.path().join("photo.png");
        fs::write(&file, b"png").unwrap();
        let attachment = attachment_add(&ctx, "", &file).unwrap();
        let _unlinked = attachment_add(&ctx, "", &file).unwrap();
        let note = object_create(&ctx, None, ObjectType::Product, "Note".into()).unwrap();
        let page = object_create(&ctx, None, ObjectType::Product, "Page".into()).unwrap();
        let missing = "deadbeef-0000-4000-8000-000000000137";
        object_write(
            &ctx,
            &page,
            &format!(
                "![a](cerbo://objects/{attachment}) [n](cerbo://{note}) \
                 cerbo://{attachment} cerbo://{missing}"
            ),
        )
        .unwrap();

        assert_eq!(attachment_list(&ctx, &page).unwrap(), vec![attachment.clone()]);
        assert!(attachment_list(&ctx, &note).unwrap().is_empty());
        assert!(!object_path(&ctx, missing).exists());
    }

    /// A live page that still links to a trashed object must not bring it back.
    #[test]
    fn saving_a_link_to_a_trashed_object_does_not_resurrect_it() {
        let (_tmp, ctx) = create_test_context();
        let target = object_create(&ctx, None, ObjectType::Product, "Target".into()).unwrap();
        let source = object_create(&ctx, None, ObjectType::Product, "Source".into()).unwrap();
        object_delete(&ctx, &target).unwrap();

        object_write(&ctx, &source, &format!("cerbo://objects/{target}"))
            .expect("a broken link must not fail the save");

        assert_eq!(broken_links(&ctx, &format!("cerbo://objects/{target}")), vec![target.clone()]);
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

    // ── Standard vocabulary ───────────────────────────────────────────────

    const UUID_137: &str = "aaaaaaaa-0000-0000-0000-000000000137";

    /// The `rdf:type` objects and the triples of `meta.ttl` as parsed.
    fn parsed_meta(content: &str) -> (Vec<String>, Vec<oxrdf::Triple>) {
        let triples = crate::rdf::parse(content).expect("meta.ttl must be valid Turtle");
        let types = triples
            .iter()
            .filter(|t| t.predicate == crate::rdf::rdf_type())
            .map(|t| match &t.object {
                oxrdf::Term::NamedNode(n) => n.as_str().to_string(),
                other => panic!("rdf:type must be an IRI, got {other:?}"),
            })
            .collect();
        (types, triples)
    }

    fn object_of(triples: &[oxrdf::Triple], predicate: &str) -> Option<oxrdf::Term> {
        triples
            .iter()
            .find(|t| t.predicate.as_str() == predicate)
            .map(|t| t.object.clone())
    }

    /// Write `content` as the `meta.ttl` of an object directory named `uuid`.
    fn meta_file(ctx: &CerboContext, uuid: &str, content: &str) -> PathBuf {
        let dir = object_path(ctx, uuid);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("meta.ttl");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn page_meta_uses_schema_org_classes_and_properties() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Index".into()).unwrap();
        let content = fs::read_to_string(object_path(&ctx, &uuid).join("meta.ttl")).unwrap();
        let (types, triples) = parsed_meta(&content);

        assert_eq!(types, ["https://schema.org/NoteDigitalDocument"], "{content}");
        assert!(
            types.iter().all(|t| !t.starts_with(crate::rdf::NS_CERBO)),
            "no cerbo:// class may be written:\n{content}"
        );
        assert_eq!(
            object_of(&triples, "https://schema.org/name"),
            Some(oxrdf::Term::from(oxrdf::Literal::new_simple_literal("Index")))
        );
        for date in ["dateCreated", "dateModified"] {
            match object_of(&triples, &format!("https://schema.org/{date}")) {
                Some(oxrdf::Term::Literal(l)) => {
                    assert_eq!(l.datatype(), crate::rdf::xsd("dateTime").as_ref(), "{date}")
                }
                other => panic!("{date} must be an xsd:dateTime literal, got {other:?}"),
            }
        }
        // The subject is the page, so `?s a schema:NoteDigitalDocument ; schema:name ?n`
        // finds it with standard tools.
        assert!(
            triples
                .iter()
                .all(|t| t.subject.to_string() == format!("<cerbo://objects/{uuid}>")),
            "{content}"
        );
    }

    #[test]
    fn source_meta_is_a_read_only_web_page_with_a_url() {
        let mut meta = ObjectMeta::empty();
        meta.object_type = ObjectType::Source;
        meta.title = "Imported".into();
        meta.original_url = Some("https://example.org/137".into());
        let content = meta.to_turtle(UUID_137);
        let (types, triples) = parsed_meta(&content);

        assert_eq!(types, ["https://schema.org/WebPage"]);
        assert_eq!(
            object_of(&triples, "https://schema.org/url"),
            Some(oxrdf::Term::from(oxrdf::NamedNode::new_unchecked("https://example.org/137"))),
            "schema:url must be an IRI:\n{content}"
        );
        let read = roundtrip(&meta);
        assert_eq!(read.object_type, ObjectType::Source);
        assert!(read.object_type.is_readonly());
        assert_eq!(read.original_url.as_deref(), Some("https://example.org/137"));
    }

    #[test]
    fn attachment_meta_is_a_media_object_with_an_encoding_format() {
        let mut meta = ObjectMeta::empty();
        meta.object_type = ObjectType::Attachment;
        meta.title = "image.png".into();
        meta.mime_type = Some("image/png".into());
        let content = meta.to_turtle(UUID_137);
        let (types, triples) = parsed_meta(&content);

        assert_eq!(types, ["https://schema.org/MediaObject"]);
        assert_eq!(
            object_of(&triples, "https://schema.org/encodingFormat"),
            Some(oxrdf::Term::from(oxrdf::Literal::new_simple_literal("image/png")))
        );
        assert_eq!(roundtrip(&meta).mime_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn ontology_meta_is_a_defined_term_set() {
        let mut meta = ObjectMeta::empty();
        meta.object_type = ObjectType::Ontology;
        let (types, _) = parsed_meta(&meta.to_turtle(UUID_137));
        assert_eq!(types, ["https://schema.org/DefinedTermSet"]);
        assert_eq!(roundtrip(&meta).object_type, ObjectType::Ontology);
    }

    #[test]
    fn a_url_literal_is_accepted_on_read() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("meta.ttl");
        fs::write(
            &path,
            format!(
                "<cerbo://objects/{UUID_137}> a <https://schema.org/WebPage> ;\n  <https://schema.org/url> \"https://example.org/137\" .\n"
            ),
        )
        .unwrap();
        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.object_type, ObjectType::Source);
        assert_eq!(meta.original_url.as_deref(), Some("https://example.org/137"));
    }

    /// `test-vault/.cerbo/objects/ba079793-…/meta.ttl` as written before the
    /// vocabulary change, verbatim.
    const TEST_VAULT_LEGACY_META: &str = concat!(
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        "@prefix schema: <cerbo://ontology/schema/> .\n",
        "@prefix cerbo: <cerbo://ontology/> .\n",
        "<cerbo://objects/ba079793-b2ff-4ca6-ba88-8764ce6668c3> cerbo:type cerbo:Product ;\n",
        "\tcerbo:title \"Index\" ;\n",
        "\tschema:dateCreated \"2026-10-25T13:17:54.772697296+00:00\"^^xsd:dateTime ;\n",
        "\tschema:dateModified \"2026-10-25T13:18:16.608529895+00:00\"^^xsd:dateTime ;\n",
        "\tcerbo:slug \"index\" .\n",
    );

    #[test]
    fn legacy_test_vault_meta_reads_with_the_same_type_title_and_dates() {
        let (_tmp, ctx) = create_test_context();
        let uuid = "ba079793-b2ff-4ca6-ba88-8764ce6668c3";
        let path = meta_file(&ctx, uuid, TEST_VAULT_LEGACY_META);

        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.object_type, ObjectType::Product);
        assert_eq!(meta.title, "Index");
        assert_eq!(meta.created, "2026-10-25T13:17:54.772697296+00:00");
        assert_eq!(meta.modified, "2026-10-25T13:18:16.608529895+00:00");
        assert_eq!(meta.slug.as_deref(), Some("index"));

        // The next write uses only the standard forms.
        meta.write_to_file(&path, uuid).unwrap();
        let rewritten = fs::read_to_string(&path).unwrap();
        for legacy in ["cerbo:type", "cerbo:title", "cerbo:Product", "cerbo://ontology/schema/"] {
            assert!(!rewritten.contains(legacy), "{legacy} written back:\n{rewritten}");
        }
        let (types, _) = parsed_meta(&rewritten);
        assert_eq!(types, ["https://schema.org/NoteDigitalDocument"]);

        let reread = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(reread.title, meta.title);
        assert_eq!(reread.created, meta.created);
        assert_eq!(reread.modified, meta.modified);
        assert_eq!(reread.slug, meta.slug);
    }

    #[test]
    fn ambiguous_object_class_is_refused_and_names_the_uuid_and_classes() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, Some(UUID_137.into()), ObjectType::Product, "Both".into())
            .unwrap();
        let path = object_path(&ctx, &uuid).join("meta.ttl");
        let mut content = fs::read_to_string(&path).unwrap();
        content.push_str(&format!(
            "<cerbo://objects/{uuid}> a <https://schema.org/WebPage> .\n"
        ));
        fs::write(&path, &content).unwrap();
        let page_before = fs::read(object_path(&ctx, &uuid).join("page.md")).unwrap();

        let err = ObjectMeta::read_from_file(&path).unwrap_err().to_string();
        assert!(err.contains(&uuid), "error must name the UUID: {err}");
        assert!(err.contains("https://schema.org/NoteDigitalDocument"), "{err}");
        assert!(err.contains("https://schema.org/WebPage"), "{err}");

        // Neither class wins: the object is not written.
        let write = object_write(&ctx, &uuid, "# changed\n").unwrap_err();
        assert!(write.contains(&uuid), "{write}");
        assert_eq!(fs::read(object_path(&ctx, &uuid).join("page.md")).unwrap(), page_before);
        assert_eq!(fs::read_to_string(&path).unwrap(), content, "meta.ttl must be untouched");
    }

    #[test]
    fn ambiguity_counts_legacy_and_standard_spellings_of_the_same_class_once() {
        let (_tmp, ctx) = create_test_context();
        let path = meta_file(
            &ctx,
            UUID_137,
            &format!(
                "<cerbo://objects/{UUID_137}> a <https://schema.org/NoteDigitalDocument> ;\n  <cerbo://ontology/type> <cerbo://ontology/Product> .\n"
            ),
        );
        assert_eq!(ObjectMeta::read_from_file(&path).unwrap().object_type, ObjectType::Product);
    }

    #[test]
    fn extra_unrelated_class_keeps_a_note_a_note() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, Some(UUID_137.into()), ObjectType::Product, "Soup".into())
            .unwrap();
        let path = object_path(&ctx, &uuid).join("meta.ttl");
        let mut content = fs::read_to_string(&path).unwrap();
        content.push_str(&format!("<cerbo://objects/{uuid}> a <https://schema.org/Recipe> .\n"));
        fs::write(&path, content).unwrap();

        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.object_type, ObjectType::Product);
        assert_eq!(meta.title, "Soup");
        object_write(&ctx, &uuid, "# Soup\n\nstill a note\n").unwrap();
    }

    #[test]
    fn saving_keeps_the_triples_cerbo_does_not_model() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, Some(UUID_137.into()), ObjectType::Product, "Soup".into())
            .unwrap();
        let path = object_path(&ctx, &uuid).join("meta.ttl");
        let mut content = fs::read_to_string(&path).unwrap();
        content.push_str(&format!(
            "<cerbo://objects/{uuid}> a <https://schema.org/Recipe> ;\n  <https://example.org/rating> \"137\" .\n\
             <https://example.org/137> <https://schema.org/name> \"Linked\" .\n"
        ));
        fs::write(&path, content).unwrap();

        object_write(&ctx, &uuid, "# Soup\n\nedited\n").unwrap();
        crate::page::page_update_title(&ctx, uuid.clone(), "Stew".into()).unwrap();

        let (types, triples) = parsed_meta(&fs::read_to_string(&path).unwrap());
        assert!(types.contains(&"https://schema.org/Recipe".to_string()), "{types:?}");
        assert!(types.contains(&"https://schema.org/NoteDigitalDocument".to_string()));
        assert!(
            triples.iter().any(|t| t.predicate.as_str() == "https://example.org/rating"),
            "unmodelled triple lost: {triples:?}"
        );
        assert!(
            triples.iter().any(|t| t.object == oxrdf::Literal::new_simple_literal("Linked").into()),
            "triple about another resource lost: {triples:?}"
        );
        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.title, "Stew");

        // Carrying them over is still byte-stable.
        let first = fs::read(&path).unwrap();
        meta.write_to_file(&path, &uuid).unwrap();
        assert_eq!(first, fs::read(&path).unwrap());
    }

    #[test]
    fn a_class_given_to_another_resource_is_not_the_objects_class() {
        let (_tmp, ctx) = create_test_context();
        let uuid = object_create(&ctx, Some(UUID_137.into()), ObjectType::Product, "Links".into())
            .unwrap();
        let path = object_path(&ctx, &uuid).join("meta.ttl");
        let mut content = fs::read_to_string(&path).unwrap();
        content.push_str(
            "<https://example.org/137> a <https://schema.org/WebPage> ;\n  <https://schema.org/name> \"Elsewhere\" .\n",
        );
        fs::write(&path, content).unwrap();

        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.object_type, ObjectType::Product);
        assert_eq!(meta.title, "Links");
        object_write(&ctx, &uuid, "# Links\n").unwrap();
    }

    #[test]
    fn a_pre_turtle_meta_with_two_classes_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().join(UUID_137);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("meta.ttl");
        fs::write(
            &path,
            concat!(
                "@prefix : <cerbo://ontology/> .\n",
                "<cerbo://objects/placeholder>\n",
                "    :type :Source ;\n",
                "    :type :Product ;\n",
                "    :title \"Both\" .\n",
                "    cerbo:slug \"both\" .\n",
            ),
        )
        .unwrap();

        let err = ObjectMeta::read_from_file(&path).unwrap_err().to_string();
        assert!(err.contains(UUID_137), "{err}");
        assert!(err.contains("https://schema.org/WebPage"), "{err}");
        assert!(err.contains("https://schema.org/NoteDigitalDocument"), "{err}");
    }

}
