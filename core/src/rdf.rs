//! Cerbo's RDF vocabulary and the only place that talks to a Turtle library.
//!
//! Every `.ttl` file in a vault is built from `oxrdf` triples and serialised by
//! `oxttl`, so escaping, prefix declaration and statement termination are the
//! library's problem rather than ours — no `format!`-built RDF anywhere.

use oxrdf::{Literal, NamedNode, Triple};
use oxttl::{TurtleParser, TurtleSerializer};

/// Cerbo's own vocabulary: `:type`, `:title`, `:slug`, `:hasBacklink`, …
pub const NS_CERBO: &str = "cerbo://ontology/";
/// Schema-ish terms Cerbo mints itself: `schema:dateCreated`, `schema:dateModified`.
pub const NS_SCHEMA: &str = "cerbo://ontology/schema/";
/// XML Schema datatypes, used for `xsd:dateTime` literals.
pub const NS_XSD: &str = "http://www.w3.org/2001/XMLSchema#";
/// The object store: every object's subject IRI lives here.
pub const NS_OBJECTS: &str = "cerbo://objects/";

/// A term in Cerbo's own vocabulary.
pub fn cerbo(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{NS_CERBO}{local}"))
}

/// A term in Cerbo's schema vocabulary.
pub fn schema(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{NS_SCHEMA}{local}"))
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

/// Parse a Turtle document.
///
/// `cerbo:` is pre-declared because vaults written before this change used the
/// prefix without declaring it. Documents from that era may still fail here for
/// other reasons — callers that must read them keep a legacy fallback.
pub fn parse(content: &str) -> Result<Vec<Triple>, String> {
    let parser = TurtleParser::new()
        .with_prefix("cerbo", NS_CERBO)
        .map_err(|e| e.to_string())?
        .with_prefix("schema", NS_SCHEMA)
        .map_err(|e| e.to_string())?;

    let mut triples = Vec::new();
    for triple in parser.for_slice(content) {
        triples.push(triple.map_err(|e| e.to_string())?);
    }
    Ok(triples)
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
            cerbo("title"),
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
            cerbo("title"),
            Term::from(Literal::new_simple_literal("Stable")),
        )];
        assert_eq!(serialize(&triples), serialize(&triples));
    }

    #[test]
    fn declares_every_prefix_it_uses() {
        let doc = serialize(&[
            Triple::new(
                object_iri("137"),
                cerbo("type"),
                Term::from(cerbo("Product")),
            ),
            Triple::new(
                object_iri("137"),
                schema("dateCreated"),
                Term::from(date_time("2024-01-01T00:00:00Z")),
            ),
        ]);
        for prefix in ["@prefix cerbo:", "@prefix schema:", "@prefix xsd:"] {
            assert!(doc.contains(prefix), "missing {prefix} in:\n{doc}");
        }
        assert!(parse(&doc).is_ok(), "own output must parse:\n{doc}");
    }
}

