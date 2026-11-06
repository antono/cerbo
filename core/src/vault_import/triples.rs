//! Frontmatter to RDF in the import's namespace `cerbo://vocab/<uuid>/`
//! (design D6). Each key becomes `<ns><key>`; values that hold wikilinks become
//! object relations, others typed literals.

use super::dialect::{Dialect, SpecialKey};
use super::frontmatter::{FrontValue, Frontmatter};
use super::resolve::{Findings, LinkIndex, NoteEntry};
use super::rewrite::{WIKILINK, wiki_parts};
use crate::rdf;
use oxrdf::{Literal, NamedNode, Term, Triple};
use std::collections::BTreeMap;

/// Where every import's vocabulary lives. Never under `cerbo://objects/`,
/// which the index reads as links to objects.
pub const NS_VOCAB: &str = "cerbo://vocab/";

pub fn namespace_for(ontology_uuid: &str) -> String {
    format!("{NS_VOCAB}{ontology_uuid}/")
}

/// `key` as an IRI local name: letters, digits (any script), `-._~` are kept,
/// everything else percent-encoded, so `Belongs to` → `Belongs%20to` and no
/// two keys collide.
pub fn local_name(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for c in key.chars() {
        if c.is_ascii_alphanumeric()
            || matches!(c, '-' | '.' | '_' | '~')
            || (!c.is_ascii() && c.is_alphanumeric())
        {
            out.push(c);
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// The classes and properties an import uses, for its Ontology object.
#[derive(Debug, Default)]
pub struct Vocab {
    pub namespace: String,
    /// Local name → (label, pages of that class).
    pub classes: BTreeMap<String, (String, usize)>,
    /// Local name → (frontmatter key, triples using it).
    pub properties: BTreeMap<String, (String, usize)>,
}

impl Vocab {
    pub fn new(namespace: String) -> Self {
        Self {
            namespace,
            ..Self::default()
        }
    }

    pub fn term(&self, local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("{}{local}", self.namespace))
    }

    /// A class named `label`, declared (Tolaria `type: Type`) or used.
    pub fn class(&mut self, label: &str, used: bool) -> NamedNode {
        let local = local_name(label);
        let entry = self
            .classes
            .entry(local.clone())
            .or_insert_with(|| (label.to_string(), 0));
        if used {
            entry.1 += 1;
        }
        self.term(&local)
    }

    fn property(&mut self, key: &str) -> NamedNode {
        let local = local_name(key);
        self.properties
            .entry(local.clone())
            .or_insert_with(|| (key.to_string(), 0))
            .1 += 1;
        self.term(&local)
    }

    /// `rdfs:Class` / `rdf:Property` declarations with labels, written into
    /// the Ontology object's `meta.ttl`.
    pub fn declarations(&self) -> Vec<Triple> {
        let rdfs =
            |l: &str| NamedNode::new_unchecked(format!("http://www.w3.org/2000/01/rdf-schema#{l}"));
        let mut triples = Vec::new();
        for (local, (label, _)) in &self.classes {
            let class = self.term(local);
            triples.push(Triple::new(
                class.clone(),
                rdf::rdf_type(),
                Term::from(rdfs("Class")),
            ));
            triples.push(Triple::new(
                class,
                rdfs("label"),
                Term::from(Literal::new_simple_literal(label)),
            ));
        }
        for (local, (key, _)) in &self.properties {
            let property = self.term(local);
            let rdf_property = NamedNode::new_unchecked(format!("{}Property", rdf::NS_RDF));
            triples.push(Triple::new(
                property.clone(),
                rdf::rdf_type(),
                Term::from(rdf_property),
            ));
            triples.push(Triple::new(
                property,
                rdfs("label"),
                Term::from(Literal::new_simple_literal(key)),
            ));
        }
        triples
    }
}

/// A string value as an RDF literal: ISO dates and date-times typed as such,
/// anything else `xsd:string`.
pub fn text_literal(value: &str) -> Literal {
    let v = value.trim();
    if v.len() == 10 && chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok() {
        return Literal::new_typed_literal(v, rdf::xsd("date"));
    }
    if let Ok(t) = chrono::DateTime::parse_from_rfc3339(v) {
        // chrono also takes `t` or a space as separator, `z`, and leap second
        // 60; xsd:dateTime takes none of these.
        if t.timestamp_subsec_nanos() >= 1_000_000_000 {
            return Literal::new_simple_literal(value);
        }
        let lexical = if v.as_bytes()[10] == b'T' && !v.ends_with('z') {
            v.to_string()
        } else {
            t.to_rfc3339()
        };
        return Literal::new_typed_literal(lexical, rdf::xsd("dateTime"));
    }
    for format in [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(t) = chrono::NaiveDateTime::parse_from_str(v, format) {
            return Literal::new_typed_literal(
                t.format("%Y-%m-%dT%H:%M:%S").to_string(),
                rdf::xsd("dateTime"),
            );
        }
    }
    Literal::new_simple_literal(value)
}

fn scalar_literal(value: &FrontValue) -> Option<Literal> {
    Some(match value {
        FrontValue::Null => return None,
        FrontValue::Bool(b) => Literal::new_typed_literal(b.to_string(), rdf::xsd("boolean")),
        FrontValue::Int(i) => Literal::new_typed_literal(i.to_string(), rdf::xsd("integer")),
        FrontValue::Float(f) if f.is_finite() => {
            Literal::new_typed_literal(f.to_string(), rdf::xsd("decimal"))
        }
        FrontValue::Float(f) => Literal::new_simple_literal(f.to_string()),
        FrontValue::Str(s) => text_literal(s),
        // Nested structures are kept as their YAML text.
        FrontValue::List(_) | FrontValue::Map(_) => Literal::new_simple_literal(value.to_yaml()),
    })
}

/// The items a key's value stands for: a list's items, or the value itself.
fn items(value: &FrontValue) -> Vec<&FrontValue> {
    match value {
        FrontValue::List(items) if value.unquoted_wikilink().is_none() => items.iter().collect(),
        other => vec![other],
    }
}

/// Plain text of an item for keyword-like keys: tags, aliases, type names.
fn item_texts(value: &FrontValue, split_commas: bool) -> Vec<String> {
    let mut out = Vec::new();
    for item in items(value) {
        let Some(text) = item.unquoted_wikilink().or_else(|| item.scalar_text()) else {
            continue;
        };
        if split_commas {
            out.extend(
                text.split([',', ' '])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        } else if !text.trim().is_empty() {
            out.push(text.trim().to_string());
        }
    }
    out
}

/// Obsidian aliases of a note, for the resolution index.
pub fn aliases(front: &Frontmatter, dialect: &dyn Dialect) -> Vec<String> {
    front
        .iter()
        .filter(|(k, _)| dialect.special_key(k) == Some(SpecialKey::AlternateName))
        .flat_map(|(_, v)| item_texts(v, false))
        .collect()
}

/// Type names in a Tolaria `type` value (`Journal`, or `[[Journal]]`).
pub fn type_names(value: &FrontValue) -> Vec<String> {
    item_texts(value, false)
        .into_iter()
        .map(|t| match WIKILINK.captures(&t) {
            Some(c) => wiki_parts(&c[1]).target.to_string(),
            None => t,
        })
        .filter(|t| !t.is_empty())
        .collect()
}

/// The triples `note`'s frontmatter adds to its `meta.ttl`.
pub fn frontmatter_triples(
    front: &Frontmatter,
    dialect: &dyn Dialect,
    note: &NoteEntry,
    index: &LinkIndex,
    findings: &mut Findings,
    vocab: &mut Vocab,
) -> Vec<Triple> {
    let subject = rdf::object_iri(&note.uuid);
    let mut triples = Vec::new();
    let mut add = |predicate: NamedNode, object: Term| {
        triples.push(Triple::new(subject.clone(), predicate, object))
    };

    for (key, value) in front {
        if dialect.drop_key(key) {
            continue;
        }
        match dialect.special_key(key) {
            Some(SpecialKey::Type) => {
                for name in type_names(value) {
                    add(rdf::rdf_type(), Term::from(vocab.class(&name, true)));
                }
            }
            Some(SpecialKey::Keywords) => {
                for tag in item_texts(value, true) {
                    let tag = tag.trim_start_matches('#');
                    if !tag.is_empty() {
                        add(
                            rdf::schema("keywords"),
                            Term::from(Literal::new_simple_literal(tag)),
                        );
                    }
                }
            }
            Some(SpecialKey::AlternateName) => {
                for alias in item_texts(value, false) {
                    add(
                        rdf::schema("alternateName"),
                        Term::from(Literal::new_simple_literal(alias)),
                    );
                }
            }
            None => {
                for item in items(value) {
                    let text = item.unquoted_wikilink().or_else(|| match item {
                        FrontValue::Str(s) => Some(s.clone()),
                        _ => None,
                    });
                    match text.filter(|t| WIKILINK.is_match(t)) {
                        Some(text) => {
                            for caps in WIKILINK.captures_iter(&text) {
                                let link = &caps[0];
                                let object = match index
                                    .wikilink(wiki_parts(&caps[1]).target, &note.folder)
                                {
                                    Some(res) => {
                                        findings.resolved(index, &note.rel, link, &res);
                                        findings.relations += 1;
                                        Term::from(rdf::object_iri(index.uuid_of(res.target)))
                                    }
                                    None => {
                                        findings.unresolved(&note.rel, link, Some(key));
                                        Term::from(Literal::new_simple_literal(link))
                                    }
                                };
                                add(vocab.property(key), object);
                            }
                        }
                        None => {
                            if let Some(literal) = scalar_literal(item) {
                                add(vocab.property(key), Term::from(literal));
                            }
                        }
                    }
                }
            }
        }
    }
    triples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault_import::dialect::{Obsidian, Tolaria};
    use crate::vault_import::frontmatter;

    const NS: &str = "cerbo://vocab/00000000-0000-4000-8000-000000000137/";

    fn index() -> LinkIndex {
        let note = |rel: &str, title: &str, uuid: &str| NoteEntry {
            rel: rel.into(),
            folder: String::new(),
            title: title.into(),
            uuid: uuid.into(),
        };
        LinkIndex::new(
            vec![
                note(
                    "3d-printing.md",
                    "3D Printing",
                    "11111111-0000-4000-8000-000000000137",
                ),
                note(
                    "devices.md",
                    "Devices",
                    "22222222-0000-4000-8000-000000000137",
                ),
                note("x.md", "X", "33333333-0000-4000-8000-000000000137"),
            ],
            &[],
            vec![],
        )
    }

    fn triples(yaml: &str, dialect: &dyn Dialect) -> (Vec<String>, Findings, Vocab) {
        let ix = index();
        let mut findings = Findings::default();
        let mut vocab = Vocab::new(NS.to_string());
        let front = frontmatter::parse(yaml).unwrap();
        let note = ix.notes[2].clone();
        let out = frontmatter_triples(&front, dialect, &note, &ix, &mut findings, &mut vocab);
        let lines = out
            .iter()
            .map(|t| format!("{} {}", t.predicate, t.object))
            .collect();
        (lines, findings, vocab)
    }

    #[test]
    fn type_becomes_second_rdf_type() {
        let (lines, _, vocab) = triples("type: Journal\n", &Tolaria);
        assert_eq!(
            lines,
            [format!(
                "<{}> <{NS}Journal>",
                rdf::NS_RDF.to_string() + "type"
            )]
        );
        assert_eq!(vocab.classes["Journal"], ("Journal".to_string(), 1));
    }

    #[test]
    fn relationship_string_and_list() {
        let (lines, f, _) = triples(
            "belongs_to: \"[[3d-printing]]\"\nrelated_to:\n  - \"[[devices]]\"\n  - \"[[3D Printing]]\"\n",
            &Tolaria,
        );
        assert_eq!(
            lines,
            [
                format!("<{NS}belongs_to> <cerbo://objects/11111111-0000-4000-8000-000000000137>"),
                format!("<{NS}related_to> <cerbo://objects/22222222-0000-4000-8000-000000000137>"),
                format!("<{NS}related_to> <cerbo://objects/11111111-0000-4000-8000-000000000137>"),
            ]
        );
        assert_eq!(f.relations, 3);
    }

    #[test]
    fn unquoted_wikilink_relationship() {
        let (lines, _, _) = triples("belongs_to: [[devices]]\n", &Tolaria);
        assert_eq!(
            lines,
            [format!(
                "<{NS}belongs_to> <cerbo://objects/22222222-0000-4000-8000-000000000137>"
            )]
        );
    }

    #[test]
    fn unresolved_relationship_becomes_literal() {
        let (lines, f, _) = triples("related_to: \"[[Missing Thing]]\"\n", &Tolaria);
        assert_eq!(lines, [format!("<{NS}related_to> \"[[Missing Thing]]\"")]);
        assert_eq!(f.unresolved[0].property.as_deref(), Some("related_to"));
        assert_eq!(f.relations, 0);
    }

    #[test]
    fn literals_with_datatypes() {
        let (lines, _, _) = triples(
            "status: Active\nurl: https://example.com\ndone: true\ncount: 137\ndate: 2025-04-27\nat: 2025-04-27 10:30\nmeta:\n  a: 1\n",
            &Tolaria,
        );
        let xsd = rdf::NS_XSD;
        assert_eq!(
            lines,
            [
                format!("<{NS}status> \"Active\""),
                format!("<{NS}url> \"https://example.com\""),
                format!("<{NS}done> \"true\"^^<{xsd}boolean>"),
                format!("<{NS}count> \"137\"^^<{xsd}integer>"),
                format!("<{NS}date> \"2025-04-27\"^^<{xsd}date>"),
                format!("<{NS}at> \"2025-04-27T10:30:00\"^^<{xsd}dateTime>"),
                format!("<{NS}meta> \"a: 1\""),
            ]
        );
    }

    #[test]
    fn date_times_are_valid_xsd() {
        let xsd = rdf::NS_XSD;
        let typed = |v: &str| format!("\"{v}\"^^<{xsd}dateTime>");
        assert_eq!(
            text_literal("2025-04-27T10:30:00+02:00").to_string(),
            typed("2025-04-27T10:30:00+02:00")
        );
        assert_eq!(
            text_literal("2025-04-27 10:30:00+02:00").to_string(),
            typed("2025-04-27T10:30:00+02:00")
        );
        assert_eq!(
            text_literal("2025-04-27t10:30:00Z").to_string(),
            typed("2025-04-27T10:30:00+00:00")
        );
        // A leap second has no xsd:dateTime form.
        assert_eq!(
            text_literal("2016-12-31T23:59:60Z").to_string(),
            "\"2016-12-31T23:59:60Z\""
        );
    }

    #[test]
    fn big_integers_are_xsd_integer() {
        let (lines, _, _) = triples("id: 18446744073709551615\n", &Tolaria);
        assert_eq!(
            lines,
            [format!(
                "<{NS}id> \"18446744073709551615\"^^<{}integer>",
                rdf::NS_XSD
            )]
        );
    }

    #[test]
    fn list_items_are_one_triple_each() {
        let (lines, _, _) = triples("tags: [hardware, keebs]\n", &Tolaria);
        assert_eq!(
            lines,
            [
                format!("<{NS}tags> \"hardware\""),
                format!("<{NS}tags> \"keebs\"")
            ]
        );
    }

    #[test]
    fn app_state_is_dropped() {
        let (lines, _, vocab) = triples(
            "_favorite: true\n_favorite_index: 4\nstatus: Active\n",
            &Tolaria,
        );
        assert_eq!(lines.len(), 1);
        assert!(!vocab.properties.contains_key("_favorite"));
    }

    #[test]
    fn obsidian_tags_and_aliases() {
        let (lines, _, _) = triples("tags: [rust, \"#lang\"]\naliases: [Ownership]\n", &Obsidian);
        assert_eq!(
            lines,
            [
                "<https://schema.org/keywords> \"rust\"",
                "<https://schema.org/keywords> \"lang\"",
                "<https://schema.org/alternateName> \"Ownership\"",
            ]
        );
        let front = frontmatter::parse("aliases: [Ownership, Borrow checker]\n").unwrap();
        assert_eq!(aliases(&front, &Obsidian), ["Ownership", "Borrow checker"]);
    }

    #[test]
    fn keys_are_percent_encoded() {
        assert_eq!(local_name("Belongs to"), "Belongs%20to");
        assert_eq!(local_name("статус"), "статус");
        assert_eq!(local_name("a/b#c"), "a%2Fb%23c");
        assert!(NamedNode::new(format!("{NS}{}", local_name("a b%c?"))).is_ok());
    }

    #[test]
    fn type_names_strip_wikilinks() {
        assert_eq!(
            type_names(&FrontValue::Str("[[Journal]]".into())),
            ["Journal"]
        );
    }
}
