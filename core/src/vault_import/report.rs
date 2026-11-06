//! The import report: what was (or, on a dry run, would be) created and what
//! needs the user's attention. Serialised camelCase for `--json`.

use super::Format;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// The target vault; `None` on a dry run.
    pub vault: Option<ReportVault>,
    /// Canonical path of the source folder.
    pub source: String,
    pub format: Format,
    /// Why this format was used: `--from`, or what detection found.
    pub format_reason: String,
    pub dry_run: bool,
    /// Pages created.
    pub pages: usize,
    /// Attachment objects created.
    pub attachments: usize,
    /// Total size of the attachment files, in bytes.
    pub attachment_bytes: u64,
    /// Object relations from frontmatter (`<ns><key> <cerbo://objects/…>`).
    pub relations: usize,
    /// Links in page bodies rewritten to `cerbo://`.
    pub links: usize,
    /// `#Heading` fragments dropped from rewritten links.
    pub dropped_fragments: usize,
    pub ontology: ReportOntology,
    /// Files left out by the format's rules (`.canvas`, Tolaria `views/`, …).
    pub skipped: Vec<String>,
    /// Non-note files no note references; not imported.
    pub unreferenced: Vec<String>,
    pub unresolved: Vec<UnresolvedLink>,
    pub ambiguous: Vec<AmbiguousLink>,
    pub renamed_slugs: Vec<RenamedSlug>,
    /// Notes whose frontmatter could not be parsed; kept as a YAML block.
    pub frontmatter_errors: Vec<FrontmatterError>,
    /// Objects of an earlier, interrupted import moved to the trash first.
    pub recovered: usize,
    /// Problems that did not stop the import (an index refresh that failed).
    pub warnings: Vec<String>,
}

/// The vault imported into. The CLI fills in `id` and `name` from the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportVault {
    pub id: Option<String>,
    pub name: Option<String>,
    pub path: String,
}

/// The import's Ontology object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportOntology {
    pub uuid: String,
    pub title: String,
    pub namespace: String,
    /// The prefix registered for the namespace; `None` until written.
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnresolvedLink {
    /// Relative path of the note the link is in.
    pub note: String,
    /// The link as written.
    pub link: String,
    /// The frontmatter key, for a link in frontmatter.
    pub property: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbiguousLink {
    pub note: String,
    pub link: String,
    /// Relative path of the note or file the link now points to.
    pub chosen: String,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamedSlug {
    pub note: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontmatterError {
    pub note: String,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use crate::vault_import::plan_import;
    use crate::vault_import::tests::tolaria_fixture;

    #[test]
    fn json_field_names_match_the_spec() {
        let report = plan_import(&tolaria_fixture(), None).unwrap().report();
        let json = serde_json::to_value(&report).unwrap();
        let object = json.as_object().unwrap();
        for field in [
            "vault",
            "format",
            "pages",
            "attachments",
            "relations",
            "skipped",
            "unreferenced",
            "unresolved",
            "ambiguous",
            "renamedSlugs",
        ] {
            assert!(object.contains_key(field), "missing {field}: {json}");
        }
        assert_eq!(json["format"], "tolaria");
        assert_eq!(json["vault"], serde_json::Value::Null);
        assert_eq!(json["dryRun"], true);
    }
}
