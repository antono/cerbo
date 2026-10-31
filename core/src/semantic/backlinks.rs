//! Backlinks as a query over the source-side links in the index (design D11).
//! No backlink is stored anywhere: each page's graph records its own outgoing
//! `schema:citation` links, and the pages citing an object are found by query.

use super::query::QueryOutput;
use super::session::{IndexOptions, SemanticIndex};
use super::{GraphIndex, SemanticError};
use crate::rdf::{self, NS_SCHEMA};
use crate::VaultContext;
use oxigraph::model::Term;
use serde::Serialize;

/// A page that links to an object, with the title from its `meta.ttl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BacklinkEntry {
    pub uuid: String,
    pub title: String,
}

impl SemanticIndex {
    /// The live pages whose `page.md` links to `uuid`, each once, ordered by
    /// title then UUID. A page without a title is left out. `uuid` must be in
    /// the normalised form of [`super::link_targets`].
    pub fn backlinks(&self, uuid: &str) -> Result<Vec<BacklinkEntry>, SemanticError> {
        // Full IRIs: a vault's prefix map may redefine `schema:`.
        let sparql = format!(
            "SELECT DISTINCT ?src ?title WHERE {{\n\
             \x20 GRAPH ?g {{ ?src <{NS_SCHEMA}citation> <{target}> }}\n\
             \x20 ?src <{NS_SCHEMA}name> ?title\n\
             }} ORDER BY ?title ?src",
            target = rdf::object_iri(uuid).as_str(),
        );
        let QueryOutput::Solutions { rows, .. } = self.live_graph().query(&sparql, &[])? else {
            unreachable!("a SELECT always yields solutions");
        };

        let mut entries: Vec<BacklinkEntry> = Vec::with_capacity(rows.len());
        for row in rows {
            let (Some(Term::NamedNode(src)), Some(Term::Literal(title))) = (&row[0], &row[1]) else {
                continue;
            };
            let Some(source) = rdf::uuid_from_object_iri(src.as_str()) else {
                continue;
            };
            // A page with two titles yields two rows; the first, by order, wins.
            if entries.iter().any(|e| e.uuid == source) {
                continue;
            }
            entries.push(BacklinkEntry { uuid: source.to_string(), title: title.value().to_string() });
        }
        Ok(entries)
    }
}

/// The backlinks of the object `uuid`, the way the CLI asks: open (building if
/// needed), catch up with the vault files, query, release. An id that is not a
/// live object is [`SemanticError::ObjectNotFound`].
pub fn backlinks(
    vault_ctx: &VaultContext,
    uuid: &str,
    options: IndexOptions<'_>,
) -> Result<Vec<BacklinkEntry>, SemanticError> {
    let not_found = || SemanticError::ObjectNotFound(uuid.to_string());
    let uuid = uuid::Uuid::parse_str(uuid).map_err(|_| not_found())?.hyphenated().to_string();
    if !vault_ctx.object_path(&uuid).is_dir() {
        return Err(not_found());
    }
    let mut index = SemanticIndex::open(vault_ctx, options)?;
    index.catch_up(options.progress)?;
    index.backlinks(&uuid)
}
