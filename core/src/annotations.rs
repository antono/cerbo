use crate::{CerboContext, object};
use regex::Regex;
use std::fs;

// ── Annotation Extraction ──────────────────────────────────────

/// Annotation extracted from page content
#[derive(Debug, Clone)]
pub struct Annotation {
    pub text: String,
    pub prefix: String,
    pub type_name: String,
    pub line: usize,
    pub column: usize,
}

/// Extract HackMD annotations from page content: [Text]{prefix:Type}
pub fn extract_annotations(content: &str) -> Vec<Annotation> {
    let re = Regex::new(r"\[([^\]]+)\]\{([^}]+)\}").unwrap();
    let mut annotations = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for cap in re.captures_iter(line) {
            if let (Some(text_cap), Some(type_cap)) = (cap.get(1), cap.get(2)) {
                let full_type = type_cap.as_str().trim();
                // Parse "prefix:Type" or just "Type"
                let (prefix, type_name) = if let Some(pos) = full_type.find(':') {
                    let (p, t) = full_type.split_at(pos);
                    (p.trim().to_string(), t[1..].trim().to_string())
                } else {
                    ("".to_string(), full_type.to_string())
                };

                annotations.push(Annotation {
                    text: text_cap.as_str().to_string(),
                    prefix,
                    type_name,
                    line: line_num + 1,
                    column: text_cap.start(),
                });
            }
        }
    }

    annotations
}

// ── Annotations.ttl Management ────────────────────────────────────

/// Write annotations.ttl for an object
pub fn annotations_write(ctx: &CerboContext, uuid: &str, annotations: &[Annotation]) -> Result<(), String> {
    let obj_dir = object::object_path(ctx, uuid);
    let annotations_path = obj_dir.join("annotations.ttl");

    let mut lines = vec![
        "@prefix : <cerbo://ontology/> .".to_string(),
        "".to_string(),
        "<cerbo://objects/<uuid>>".to_string(),
    ];

    for (i, ann) in annotations.iter().enumerate() {
        // Resolve prefix to full URI using ontology-map.json
        let type_uri = resolve_prefix_to_uri(ctx, &ann.prefix, &ann.type_name);

        lines.push(format!(
            "    :annotation [ :concept \"{}\" ; :type <{}> ; :position \"{},{}\" ] ;",
            ann.text, type_uri, ann.line, ann.column
        ));

        if i == annotations.len() - 1 {
            // Last one gets the period
            if let Some(last) = lines.last_mut() {
                *last = last.replace(" ;", " .");
            }
        }
    }

    if annotations.is_empty() {
        lines.push("    :annotation [ :concept \"none\" ] .".to_string());
    }

    let content = lines.join("\n") + "\n";

    fs::write(&annotations_path, content)
        .map_err(|e| format!("Failed to write annotations.ttl: {}", e))
}

/// Read annotations.ttl (placeholder - full parsing not implemented)
pub fn annotations_read(ctx: &CerboContext, uuid: &str) -> Result<Vec<Annotation>, String> {
    let obj_dir = object::object_path(ctx, uuid);
    let annotations_path = obj_dir.join("annotations.ttl");

    if !annotations_path.exists() {
        return Ok(Vec::new());
    }

    // Full parsing would require proper Turtle parsing
    // For now, return empty vec
    Ok(Vec::new())
}

/// Resolve prefix:Type to full URI using ontology-map.json
fn resolve_prefix_to_uri(ctx: &CerboContext, prefix: &str, type_name: &str) -> String {
    if prefix.is_empty() {
        return format!("cerbo://ontology/{}", type_name);
    }

    let map_path = ctx.config_dir.join("ontology-map.json");
    if !map_path.exists() {
        return format!("cerbo://ontology/{}", type_name);
    }

    let content = match fs::read_to_string(&map_path) {
        Ok(c) => c,
        Err(_) => return format!("cerbo://ontology/{}", type_name),
    };

    // Parse {"prefixes": {"schema": "<uuid>", ...}}
    #[derive(serde::Deserialize)]
    struct OntologyMap {
        prefixes: std::collections::HashMap<String, String>,
    }

    let map: OntologyMap = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return format!("cerbo://ontology/{}", type_name),
    };

    if let Some(uuid) = map.prefixes.get(prefix) {
        format!("cerbo://objects/{}/{}", uuid, type_name)
    } else {
        format!("cerbo://ontology/{}", type_name)
    }
}

// ── Integration with Page Write ──────────────────────────────────────

/// Wrapper around object_write that also extracts and writes annotations
pub fn page_write_with_annotations(ctx: &CerboContext, uuid: &str, content: &str) -> Result<(), String> {
    // Write content first
    object::object_write(ctx, uuid, content)?;

    // Extract annotations
    let annotations = extract_annotations(content);

    // Write to annotations.ttl
    if !annotations.is_empty() {
        annotations_write(ctx, uuid, &annotations)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_annotations() {
        let content = "This is [Bob]{schema:Person} and [Alice]{foaf:Person}.";
        let annotations = extract_annotations(content);

        assert_eq!(annotations.len(), 2);

        assert_eq!(annotations[0].text, "Bob");
        assert_eq!(annotations[0].prefix, "schema");
        assert_eq!(annotations[0].type_name, "Person");

        assert_eq!(annotations[1].text, "Alice");
        assert_eq!(annotations[1].prefix, "foaf");
        assert_eq!(annotations[1].type_name, "Person");
    }

    #[test]
    fn test_extract_annotations_no_prefix() {
        let content = "This is [Bob]{Person}.";
        let annotations = extract_annotations(content);

        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].prefix, "");
        assert_eq!(annotations[0].type_name, "Person");
    }

    #[test]
    fn test_extract_annotations_none() {
        let content = "This is plain text.";
        let annotations = extract_annotations(content);
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_resolve_prefix_to_uri() {
        let ctx = crate::CerboContext {
            config_dir: "/tmp/test_annotations".into(),
            cache_dir: "/tmp/test_annotations/cache".into(),
        };

        // Without ontology-map.json, should return default URI
        let uri = resolve_prefix_to_uri(&ctx, "schema", "Person");
        assert!(uri.contains("cerbo://ontology/"));

        // TODO: Test with actual ontology-map.json
    }
}
