//! Cerbo's RDF vocabulary and the only place that talks to a Turtle library.
//!
//! Every `.ttl` file in a vault is built from `oxrdf` triples and serialised by
//! `oxttl`, so escaping, prefix declaration and statement termination are the
//! library's problem rather than ours — no `format!`-built RDF anywhere.
//!
//! Cerbo speaks schema.org wherever schema.org has a term, and keeps its own
//! `cerbo:` namespace only for what schema.org lacks (`slug`, `virtualPath`,
//! `position`, …). Vaults written before that switch used private spellings;
//! [`parse`] rewrites them through [`LEGACY_PREDICATES`] and [`LEGACY_CLASSES`]
//! so no reader ever has to know them.

use oxrdf::{Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use oxttl::{TurtleParser, TurtleSerializer};

/// Cerbo's own vocabulary: `:slug`, `:virtualPath`, `:position`, `:hasBacklink`, …
pub const NS_CERBO: &str = "cerbo://ontology/";
/// schema.org: object classes, `schema:name`, `schema:dateCreated`, `schema:url`, …
pub const NS_SCHEMA: &str = "https://schema.org/";
/// The RDF vocabulary itself, for `rdf:type`.
pub const NS_RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
/// XML Schema datatypes, used for `xsd:dateTime` literals.
pub const NS_XSD: &str = "http://www.w3.org/2001/XMLSchema#";
/// The object store: every object's subject IRI lives here.
pub const NS_OBJECTS: &str = "cerbo://objects/";

/// Where `schema:` pointed before Cerbo adopted schema.org. Read only: every
/// IRI under it is rewritten to the same local name under [`NS_SCHEMA`].
pub const LEGACY_NS_SCHEMA: &str = "cerbo://ontology/schema/";

/// Legacy predicate IRI → the standard IRI that replaces it. Read only.
pub const LEGACY_PREDICATES: &[(&str, &str)] = &[
    ("cerbo://ontology/type", "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
    ("cerbo://ontology/title", "https://schema.org/name"),
    ("cerbo://ontology/original-url", "https://schema.org/url"),
    ("cerbo://ontology/mime-type", "https://schema.org/encodingFormat"),
    ("cerbo://ontology/annotation", "https://schema.org/mentions"),
    ("cerbo://ontology/concept", "https://schema.org/name"),
];

/// Legacy object class → the schema.org class that replaces it. Rewritten only
/// where it is the object of an `rdf:type`. Read only.
pub const LEGACY_CLASSES: &[(&str, &str)] = &[
    ("cerbo://ontology/Product", "https://schema.org/NoteDigitalDocument"),
    ("cerbo://ontology/Source", "https://schema.org/WebPage"),
    ("cerbo://ontology/Attachment", "https://schema.org/MediaObject"),
    ("cerbo://ontology/Ontology", "https://schema.org/DefinedTermSet"),
];

/// A term in Cerbo's own vocabulary.
pub fn cerbo(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{NS_CERBO}{local}"))
}

/// A schema.org term.
pub fn schema(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{NS_SCHEMA}{local}"))
}

/// `rdf:type`.
pub fn rdf_type() -> NamedNode {
    oxrdf::vocab::rdf::TYPE.into_owned()
}

/// An XML Schema datatype.
pub fn xsd(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{NS_XSD}{local}"))
}

/// The subject IRI of the object with this UUID.
pub fn object_iri(uuid: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{NS_OBJECTS}{uuid}"))
}

/// An `xsd:dateTime` literal.
pub fn date_time(value: &str) -> Literal {
    Literal::new_typed_literal(value, xsd("dateTime"))
}

/// If `iri` names an object, return its UUID.
pub fn uuid_from_object_iri(iri: &str) -> Option<&str> {
    iri.strip_prefix(NS_OBJECTS)
}

/// Serialise `triples` into a Turtle document with every prefix declared.
///
/// Output is a pure function of the triples and their order, so rewriting an
/// unchanged object is byte-stable.
pub fn serialize(triples: &[Triple]) -> String {
    let mut serializer = TurtleSerializer::new()
        .with_prefix("rdf", NS_RDF)
        .expect("rdf namespace is a valid IRI")
        .with_prefix("cerbo", NS_CERBO)
        .expect("cerbo namespace is a valid IRI")
        .with_prefix("schema", NS_SCHEMA)
        .expect("schema namespace is a valid IRI")
        .with_prefix("xsd", NS_XSD)
        .expect("xsd namespace is a valid IRI")
        .for_writer(Vec::new());

    for triple in triples {
        serializer
            .serialize_triple(triple.as_ref())
            .expect("writing to a Vec cannot fail");
    }

    let bytes = serializer.finish().expect("writing to a Vec cannot fail");
    String::from_utf8(bytes).expect("oxttl emits UTF-8")
}

/// Parse a Turtle document, upgrading legacy vocabulary to the standard one.
///
/// `cerbo:` and `schema:` are pre-declared because vaults written before the
/// Turtle rewrite used them without declaring them. An undeclared `schema:`
/// meant the legacy namespace then; [`upgrade`] maps both readings to the same
/// schema.org IRI, so the choice of pre-declaration does not matter. Documents
/// from that era may still fail here for other reasons — callers that must read
/// them keep a legacy fallback.
pub fn parse(content: &str) -> Result<Vec<Triple>, String> {
    parse_detecting_legacy(content).map(|(triples, _)| triples)
}

/// [`parse`], also saying whether any triple used a legacy spelling — how the
/// vocabulary migration tells a file that needs rewriting from one that is
/// already standard, whatever its layout.
pub fn parse_detecting_legacy(content: &str) -> Result<(Vec<Triple>, bool), String> {
    let parser = TurtleParser::new()
        .with_prefix("cerbo", NS_CERBO)
        .map_err(|e| e.to_string())?
        .with_prefix("schema", NS_SCHEMA)
        .map_err(|e| e.to_string())?;

    let mut triples = Vec::new();
    let mut legacy = false;
    for triple in parser.for_slice(content) {
        let triple = triple.map_err(|e| e.to_string())?;
        let upgraded = upgrade(triple.clone());
        legacy |= upgraded != triple;
        triples.push(upgraded);
    }
    Ok((triples, legacy))
}

/// Rewrite one triple's legacy IRIs into the standard vocabulary. A triple
/// already in standard form comes back unchanged.
pub fn upgrade(triple: Triple) -> Triple {
    let predicate = upgrade_predicate(triple.predicate);
    let subject = match triple.subject {
        NamedOrBlankNode::NamedNode(n) => NamedOrBlankNode::NamedNode(upgrade_schema_ns(n)),
        other => other,
    };
    let object = match triple.object {
        Term::NamedNode(n) => {
            let n = upgrade_schema_ns(n);
            Term::NamedNode(if predicate == oxrdf::vocab::rdf::TYPE {
                upgrade_class(n)
            } else {
                n
            })
        }
        other => other,
    };
    Triple::new(subject, predicate, object)
}

fn upgrade_predicate(predicate: NamedNode) -> NamedNode {
    match LEGACY_PREDICATES
        .iter()
        .find(|(legacy, _)| *legacy == predicate.as_str())
    {
        Some((_, standard)) => NamedNode::new_unchecked(*standard),
        None => upgrade_schema_ns(predicate),
    }
}

fn upgrade_class(class: NamedNode) -> NamedNode {
    match LEGACY_CLASSES
        .iter()
        .find(|(legacy, _)| *legacy == class.as_str())
    {
        Some((_, standard)) => NamedNode::new_unchecked(*standard),
        None => class,
    }
}

/// `cerbo://ontology/schema/<x>` → `https://schema.org/<x>`.
fn upgrade_schema_ns(node: NamedNode) -> NamedNode {
    match node.as_str().strip_prefix(LEGACY_NS_SCHEMA) {
        Some(local) => schema(local),
        None => node,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxrdf::Term;

    #[test]
    fn round_trips_a_nasty_literal() {
        let subject = object_iri("f7e435db-9740-4a2a-8e57-b439c4f8bb18");
        let nasty = "He said \"hi\"\nand C:\\temp — заметки 🎉";
        let doc = serialize(&[Triple::new(
            subject.clone(),
            schema("name"),
            Term::from(Literal::new_simple_literal(nasty)),
        )]);

        let parsed = parse(&doc).unwrap();
        assert_eq!(parsed.len(), 1);
        match &parsed[0].object {
            Term::Literal(l) => assert_eq!(l.value(), nasty),
            other => panic!("expected a literal, got {other:?}"),
        }
    }

    #[test]
    fn serialisation_is_byte_stable() {
        let triples = vec![Triple::new(
            object_iri("137"),
            schema("name"),
            Term::from(Literal::new_simple_literal("Stable")),
        )];
        assert_eq!(serialize(&triples), serialize(&triples));
    }

    #[test]
    fn declares_every_prefix_it_uses() {
        let doc = serialize(&[
            Triple::new(
                object_iri("137"),
                rdf_type(),
                Term::from(schema("NoteDigitalDocument")),
            ),
            Triple::new(
                object_iri("137"),
                schema("dateCreated"),
                Term::from(date_time("2024-01-01T00:00:00Z")),
            ),
            Triple::new(
                object_iri("137"),
                cerbo("slug"),
                Term::from(Literal::new_simple_literal("one-three-seven")),
            ),
        ]);
        for prefix in ["@prefix rdf:", "@prefix cerbo:", "@prefix schema:", "@prefix xsd:"] {
            assert!(doc.contains(prefix), "missing {prefix} in:\n{doc}");
        }
        assert!(parse(&doc).is_ok(), "own output must parse:\n{doc}");
    }

    #[test]
    fn serialised_output_uses_rdf_type_and_schema_org() {
        let triples = vec![
            Triple::new(
                object_iri("137"),
                rdf_type(),
                Term::from(schema("NoteDigitalDocument")),
            ),
            Triple::new(
                object_iri("137"),
                schema("name"),
                Term::from(Literal::new_simple_literal("Index")),
            ),
        ];
        let doc = serialize(&triples);
        assert!(doc.contains("@prefix schema: <https://schema.org/>"), "{doc}");
        assert!(
            doc.contains("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>"),
            "{doc}"
        );
        assert!(!doc.contains(LEGACY_NS_SCHEMA), "legacy namespace written:\n{doc}");

        let parsed = parse(&doc).unwrap();
        assert_eq!(parsed, triples, "own output must read back unchanged:\n{doc}");
        assert_eq!(
            parsed[0].predicate.as_str(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
        );
        assert_eq!(
            parsed[0].object,
            Term::from(NamedNode::new_unchecked("https://schema.org/NoteDigitalDocument"))
        );
    }

    #[test]
    fn parse_rewrites_every_legacy_predicate_and_class() {
        let doc = concat!(
            "@prefix cerbo: <cerbo://ontology/> .\n",
            "@prefix schema: <cerbo://ontology/schema/> .\n",
            "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
            "<cerbo://objects/137> cerbo:type cerbo:Source ;\n",
            "    cerbo:title \"Index\" ;\n",
            "    cerbo:original-url \"https://example.org/137\" ;\n",
            "    cerbo:mime-type \"text/markdown\" ;\n",
            "    cerbo:slug \"index\" ;\n",
            "    schema:dateCreated \"2024-01-01T00:00:00Z\"^^xsd:dateTime ;\n",
            "    cerbo:annotation [ cerbo:concept \"Berlin\" ; cerbo:type schema:Place ;",
            " cerbo:position \"1,10\" ] .\n",
        );
        let triples = parse(doc).unwrap();
        let predicates: Vec<&str> = triples.iter().map(|t| t.predicate.as_str()).collect();

        for legacy in LEGACY_PREDICATES.iter().map(|(l, _)| *l) {
            assert!(!predicates.contains(&legacy), "{legacy} survived parsing");
        }
        assert!(
            triples.iter().all(|t| !format!("{t}").contains(LEGACY_NS_SCHEMA)),
            "legacy schema namespace survived: {triples:?}"
        );

        let object_of = |predicate: NamedNode| -> Vec<Term> {
            triples
                .iter()
                .filter(|t| t.predicate == predicate)
                .map(|t| t.object.clone())
                .collect()
        };
        assert_eq!(
            object_of(rdf_type()),
            [Term::from(schema("WebPage")), Term::from(schema("Place"))]
        );
        assert_eq!(
            object_of(schema("name")),
            [
                Term::from(Literal::new_simple_literal("Index")),
                Term::from(Literal::new_simple_literal("Berlin")),
            ]
        );
        assert_eq!(
            object_of(schema("url")),
            [Term::from(Literal::new_simple_literal("https://example.org/137"))]
        );
        assert_eq!(
            object_of(schema("encodingFormat")),
            [Term::from(Literal::new_simple_literal("text/markdown"))]
        );
        assert_eq!(object_of(schema("dateCreated")).len(), 1);
        assert_eq!(object_of(schema("mentions")).len(), 1);
        // Terms with no schema.org equivalent stay Cerbo's.
        assert_eq!(object_of(cerbo("slug")).len(), 1);
        assert_eq!(object_of(cerbo("position")).len(), 1);
    }

    #[test]
    fn legacy_class_names_are_only_rewritten_as_types() {
        let doc = "<cerbo://objects/137> <cerbo://ontology/seeAlso> <cerbo://ontology/Product> .";
        let triples = parse(doc).unwrap();
        assert_eq!(triples[0].object, Term::from(cerbo("Product")));
    }

    #[test]
    fn detects_legacy_spellings_only_where_present() {
        let legacy = "<cerbo://objects/137> <cerbo://ontology/title> \"Index\" .";
        assert!(parse_detecting_legacy(legacy).unwrap().1);

        let standard = serialize(&[Triple::new(
            object_iri("137"),
            rdf_type(),
            Term::from(schema("NoteDigitalDocument")),
        )]);
        assert!(!parse_detecting_legacy(&standard).unwrap().1, "{standard}");
    }

    #[test]
    fn standard_documents_parse_unchanged() {
        let triples = vec![
            Triple::new(object_iri("137"), rdf_type(), Term::from(schema("Recipe"))),
            Triple::new(
                object_iri("137"),
                schema("url"),
                Term::from(NamedNode::new_unchecked("https://example.org/137")),
            ),
        ];
        assert_eq!(parse(&serialize(&triples)).unwrap(), triples);
    }
}
