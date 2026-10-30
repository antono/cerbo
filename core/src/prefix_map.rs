//! The vault's prefix map, `.cerbo/ontology-map.json`.
//!
//! Annotations such as `[Berlin]{schema:Place}` name their type with a prefix.
//! The map turns that prefix into a namespace IRI (and, optionally, the UUID of
//! the ontology object that documents it). Each vault has its own map; nothing
//! here reads the global config directory.
//!
//! Version 2 of the file:
//!
//! ```json
//! { "version": 2,
//!   "prefixes": { "ex": { "namespace": "https://example.org/vocab#", "ontology": "<uuid>" } } }
//! ```
//!
//! Earlier versions mapped a prefix straight to an ontology UUID. Such entries
//! are still read, but carry no namespace, so they resolve only through the
//! built-in table.

use crate::VaultContext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// File name of the prefix map inside a vault's `.cerbo/` directory.
pub const MAP_FILE: &str = "ontology-map.json";

/// Current on-disk version of the prefix map.
pub const MAP_VERSION: u32 = 2;

/// Well-known prefixes that resolve without any map entry. A vault entry for
/// the same prefix takes precedence.
pub const BUILTIN_PREFIXES: &[(&str, &str)] = &[
    ("schema", "https://schema.org/"),
    ("foaf", "http://xmlns.com/foaf/0.1/"),
    ("dcterms", "http://purl.org/dc/terms/"),
    ("skos", "http://www.w3.org/2004/02/skos/core#"),
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
];

/// The namespace of a built-in prefix.
pub fn builtin_namespace(prefix: &str) -> Option<&'static str> {
    BUILTIN_PREFIXES
        .iter()
        .find(|(p, _)| *p == prefix)
        .map(|(_, ns)| *ns)
}

/// One prefix in the map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefixEntry {
    /// Namespace IRI the prefix expands to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// UUID of the ontology object documenting the namespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ontology: Option<String>,
}

/// A map entry as found on disk: v2 object or v1 bare ontology UUID.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawEntry {
    Legacy(String),
    Entry(PrefixEntry),
}

#[derive(Deserialize)]
struct RawMap {
    #[serde(default)]
    prefixes: BTreeMap<String, RawEntry>,
}

#[derive(Serialize)]
struct MapFile<'a> {
    version: u32,
    prefixes: &'a BTreeMap<String, PrefixEntry>,
}

/// A vault's prefix map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrefixMap {
    pub prefixes: BTreeMap<String, PrefixEntry>,
}

impl PrefixMap {
    /// Path of the map inside a `.cerbo/` directory.
    pub fn path_in(cerbo_dir: &Path) -> PathBuf {
        cerbo_dir.join(MAP_FILE)
    }

    /// Parse the JSON content of a map file (v2, or v1 string values).
    pub fn from_json(content: &str) -> Result<Self, String> {
        let raw: RawMap = serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse {}: {}", MAP_FILE, e))?;
        let prefixes = raw
            .prefixes
            .into_iter()
            .map(|(prefix, entry)| {
                let entry = match entry {
                    RawEntry::Legacy(uuid) => PrefixEntry { namespace: None, ontology: Some(uuid) },
                    RawEntry::Entry(e) => e,
                };
                (prefix, entry)
            })
            .collect();
        Ok(Self { prefixes })
    }

    /// Load the map from a `.cerbo/` directory. A missing file is an empty map.
    pub fn load(cerbo_dir: &Path) -> Result<Self, String> {
        let path = Self::path_in(cerbo_dir);
        match std::fs::read_to_string(&path) {
            Ok(content) => Self::from_json(&content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("Failed to read {}: {}", path.display(), e)),
        }
    }

    /// Load the map of this vault.
    pub fn load_vault(vault_ctx: &VaultContext) -> Result<Self, String> {
        Self::load(&vault_ctx.cerbo_dir())
    }

    /// The v2 JSON document for this map.
    pub fn to_json(&self) -> String {
        let file = MapFile { version: MAP_VERSION, prefixes: &self.prefixes };
        serde_json::to_string_pretty(&file).expect("prefix map serialises")
    }

    /// Write the map, always as v2, with the atomic replacement protocol.
    pub fn save(&self, cerbo_dir: &Path) -> Result<(), String> {
        crate::fsio::write_atomic_str(&Self::path_in(cerbo_dir), &self.to_json())
            .map_err(|e| format!("Failed to write {}: {}", MAP_FILE, e))
    }

    /// Namespace IRI of `prefix`: the vault entry if it has one, otherwise the
    /// built-in table.
    pub fn namespace(&self, prefix: &str) -> Option<&str> {
        self.prefixes
            .get(prefix)
            .and_then(|e| e.namespace.as_deref())
            .or_else(|| builtin_namespace(prefix))
    }

    /// Full IRI of `prefix:local`, or `None` when the prefix is unknown.
    pub fn resolve(&self, prefix: &str, local: &str) -> Option<String> {
        if prefix.is_empty() {
            return None;
        }
        self.namespace(prefix).map(|ns| format!("{ns}{local}"))
    }

    /// Register `prefix` with a namespace and, optionally, its ontology object.
    pub fn register(&mut self, prefix: &str, namespace: &str, ontology: Option<&str>) {
        self.prefixes.insert(
            prefix.to_string(),
            PrefixEntry {
                namespace: Some(namespace.to_string()),
                ontology: ontology.map(str::to_string),
            },
        );
    }
}

/// Validate a prefix and namespace before registering them.
pub fn validate_registration(prefix: &str, namespace: &str) -> Result<(), String> {
    let prefix_ok = prefix
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if !prefix_ok || prefix.ends_with('.') {
        return Err(format!("Invalid prefix: {:?}", prefix));
    }
    oxrdf::NamedNode::new(namespace)
        .map_err(|e| format!("Invalid namespace IRI {:?}: {}", namespace, e))?;
    Ok(())
}

/// Add or replace one entry in the map stored in `cerbo_dir`, keeping the others.
pub fn register_prefix(
    cerbo_dir: &Path,
    prefix: &str,
    namespace: &str,
    ontology: Option<&str>,
) -> Result<(), String> {
    validate_registration(prefix, namespace)?;
    let mut map = PrefixMap::load(cerbo_dir)?;
    map.register(prefix, namespace, ontology);
    map.save(cerbo_dir)
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn vault_with_map(json: &str) -> (TempDir, VaultContext) {
        let tmp = TempDir::new().unwrap();
        let cerbo_dir = tmp.path().join(".cerbo");
        std::fs::create_dir_all(&cerbo_dir).unwrap();
        std::fs::write(cerbo_dir.join(MAP_FILE), json).unwrap();
        let ctx = VaultContext {
            vault_path: tmp.path().to_path_buf(),
            // Points elsewhere on purpose: resolution must never look here.
            global: crate::CerboContext {
                config_dir: tmp.path().join("global-config"),
                cache_dir: tmp.path().join("global-cache"),
            },
        };
        (tmp, ctx)
    }

    #[test]
    fn map_entry_resolves_to_its_namespace() {
        let (_tmp, ctx) = vault_with_map(
            r#"{"version":2,"prefixes":{"ex":{"namespace":"https://example.org/vocab#","ontology":"aaaaaaaa-0000-0000-0000-000000000137"}}}"#,
        );
        let map = PrefixMap::load_vault(&ctx).unwrap();
        assert_eq!(map.resolve("ex", "Person").as_deref(), Some("https://example.org/vocab#Person"));
        assert_eq!(
            map.prefixes["ex"].ontology.as_deref(),
            Some("aaaaaaaa-0000-0000-0000-000000000137")
        );
    }

    #[test]
    fn builtin_prefixes_resolve_with_an_empty_map() {
        let (_tmp, ctx) = vault_with_map(r#"{"version":2,"prefixes":{}}"#);
        let map = PrefixMap::load_vault(&ctx).unwrap();
        assert_eq!(map.resolve("schema", "Place").as_deref(), Some("https://schema.org/Place"));
        assert_eq!(map.resolve("foaf", "Person").as_deref(), Some("http://xmlns.com/foaf/0.1/Person"));
        assert_eq!(map.resolve("skos", "Concept").as_deref(), Some("http://www.w3.org/2004/02/skos/core#Concept"));
        assert_eq!(map.resolve("zz", "Widget"), None);
        assert_eq!(map.resolve("", "Widget"), None);
    }

    #[test]
    fn missing_map_file_is_an_empty_map() {
        let tmp = TempDir::new().unwrap();
        let map = PrefixMap::load(tmp.path()).unwrap();
        assert!(map.prefixes.is_empty());
        assert_eq!(map.resolve("schema", "Place").as_deref(), Some("https://schema.org/Place"));
    }

    #[test]
    fn vault_entry_overrides_a_builtin_prefix() {
        let (_tmp, ctx) = vault_with_map(
            r#"{"version":2,"prefixes":{"schema":{"namespace":"https://example.org/schema/"}}}"#,
        );
        let map = PrefixMap::load_vault(&ctx).unwrap();
        assert_eq!(map.resolve("schema", "Place").as_deref(), Some("https://example.org/schema/Place"));
    }

    #[test]
    fn v1_file_resolves_only_well_known_prefixes() {
        let (_tmp, ctx) = vault_with_map(
            r#"{"prefixes":{"schema":"aaaaaaaa-0000-0000-0000-000000000137","ex":"bbbbbbbb-0000-0000-0000-000000000137"}}"#,
        );
        let map = PrefixMap::load_vault(&ctx).unwrap();
        assert_eq!(map.resolve("schema", "Place").as_deref(), Some("https://schema.org/Place"));
        assert_eq!(map.resolve("ex", "Person"), None, "a v1 entry carries no namespace");
        assert_eq!(
            map.prefixes["ex"].ontology.as_deref(),
            Some("bbbbbbbb-0000-0000-0000-000000000137")
        );
    }

    #[test]
    fn two_vaults_resolve_ex_with_their_own_maps() {
        let (_a, vault_a) =
            vault_with_map(r#"{"version":2,"prefixes":{"ex":{"namespace":"https://a.example/#"}}}"#);
        let (_b, vault_b) =
            vault_with_map(r#"{"version":2,"prefixes":{"ex":{"namespace":"https://b.example/#"}}}"#);
        assert_eq!(
            PrefixMap::load_vault(&vault_a).unwrap().resolve("ex", "Thing").as_deref(),
            Some("https://a.example/#Thing")
        );
        assert_eq!(
            PrefixMap::load_vault(&vault_b).unwrap().resolve("ex", "Thing").as_deref(),
            Some("https://b.example/#Thing")
        );
    }

    #[test]
    fn register_upgrades_a_v1_file_to_v2_and_keeps_other_entries() {
        let (_tmp, ctx) = vault_with_map(r#"{"prefixes":{"foaf":"aaaaaaaa-0000-0000-0000-000000000137"}}"#);
        register_prefix(
            &ctx.cerbo_dir(),
            "ex",
            "https://example.org/vocab#",
            Some("bbbbbbbb-0000-0000-0000-000000000137"),
        )
        .unwrap();

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(ctx.cerbo_dir().join(MAP_FILE)).unwrap()).unwrap();
        assert_eq!(json["version"], 2);
        assert_eq!(json["prefixes"]["ex"]["namespace"], "https://example.org/vocab#");
        assert_eq!(json["prefixes"]["ex"]["ontology"], "bbbbbbbb-0000-0000-0000-000000000137");
        assert_eq!(json["prefixes"]["foaf"]["ontology"], "aaaaaaaa-0000-0000-0000-000000000137");
        assert!(json["prefixes"]["foaf"].get("namespace").is_none());
    }

    #[test]
    fn register_rejects_bad_prefix_or_namespace() {
        let tmp = TempDir::new().unwrap();
        assert!(register_prefix(tmp.path(), "", "https://example.org/#", None).is_err());
        assert!(register_prefix(tmp.path(), "e x", "https://example.org/#", None).is_err());
        assert!(register_prefix(tmp.path(), "ex", "not an iri", None).is_err());
        assert!(!tmp.path().join(MAP_FILE).exists());
    }
}
