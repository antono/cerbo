//! Read-only SPARQL over the index (spec `graph-query`, design D2).

use super::SemanticError;
use crate::prefix_map::{PrefixMap, BUILTIN_PREFIXES};
use crate::rdf::NS_CERBO;
use oxigraph::io::{JsonLdProfileSet, RdfFormat, RdfSerializer};
use oxigraph::model::{NamedNode, Term, Triple};
use oxigraph::sparql::results::{QueryResultsFormat, QueryResultsSerializer};
use oxigraph::sparql::{QueryResults, SparqlEvaluator, Variable};
use oxigraph::store::Store;

/// A query's complete answer, collected so the index can be closed and its
/// lock released before anyone reads the results.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryOutput {
    /// SELECT: the projected variables (without `?`) and one row per solution,
    /// with `None` where a variable is unbound.
    Solutions {
        variables: Vec<String>,
        rows: Vec<Vec<Option<Term>>>,
    },
    /// ASK.
    Boolean(bool),
    /// CONSTRUCT or DESCRIBE.
    Graph(Vec<Triple>),
}

impl QueryOutput {
    /// The answer for a terminal (spec `graph-query`): SELECT as a table with
    /// one column per variable, ASK as `true` or `false`, CONSTRUCT and
    /// DESCRIBE as a Turtle document. Ends with a newline.
    pub fn to_text(&self) -> String {
        match self {
            Self::Solutions { variables, rows } => table(variables, rows),
            Self::Boolean(b) => format!("{b}\n"),
            Self::Graph(triples) => crate::rdf::serialize(triples),
        }
    }

    /// The answer for a program: SELECT and ASK as SPARQL 1.1 Query Results
    /// JSON, CONSTRUCT and DESCRIBE as JSON-LD. Ends with a newline.
    pub fn to_json(&self) -> String {
        const IN_MEMORY: &str = "writing to a Vec cannot fail";
        let serializer = QueryResultsSerializer::from_format(QueryResultsFormat::Json);
        let mut bytes = match self {
            Self::Solutions { variables, rows } => {
                let variables: Vec<Variable> = variables.iter().map(|v| Variable::new_unchecked(v.as_str())).collect();
                let mut writer = serializer
                    .serialize_solutions_to_writer(Vec::new(), variables.clone())
                    .expect(IN_MEMORY);
                for row in rows {
                    let bound = variables
                        .iter()
                        .zip(row)
                        .filter_map(|(v, term)| term.as_ref().map(|t| (v.as_ref(), t.as_ref())));
                    writer.serialize(bound).expect(IN_MEMORY);
                }
                writer.finish().expect(IN_MEMORY)
            }
            Self::Boolean(b) => serializer.serialize_boolean_to_writer(Vec::new(), *b).expect(IN_MEMORY),
            Self::Graph(triples) => {
                let mut writer = RdfSerializer::from_format(RdfFormat::JsonLd { profile: JsonLdProfileSet::empty() })
                    .for_writer(Vec::new());
                for triple in triples {
                    writer.serialize_triple(triple).expect(IN_MEMORY);
                }
                writer.finish().expect(IN_MEMORY)
            }
        };
        bytes.push(b'\n');
        String::from_utf8(bytes).expect("both serializers emit UTF-8")
    }
}

/// A term as a person reads it: an IRI bare, a literal by its value, a blank
/// node as `_:id`. Exact terms are what `--json` is for.
fn plain(term: &Term) -> String {
    match term {
        Term::NamedNode(n) => n.as_str().to_string(),
        Term::Literal(l) => l.value().to_string(),
        other => other.to_string(),
    }
}

/// SELECT results as a left-aligned table: a header of variable names, a rule,
/// and one line per solution, with unbound cells left empty.
fn table(variables: &[String], rows: &[Vec<Option<Term>>]) -> String {
    let cells: Vec<Vec<String>> = rows
        .iter()
        .map(|row| row.iter().map(|t| t.as_ref().map(plain).unwrap_or_default()).collect())
        .collect();
    let widths: Vec<usize> = variables
        .iter()
        .enumerate()
        .map(|(i, v)| {
            cells
                .iter()
                .map(|row| row[i].chars().count())
                .chain(std::iter::once(v.chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |values: &mut dyn Iterator<Item = String>| {
        let padded: Vec<String> = values
            .zip(&widths)
            .map(|(value, width)| {
                let pad = width.saturating_sub(value.chars().count());
                format!("{value}{}", " ".repeat(pad))
            })
            .collect();
        format!("{}\n", padded.join("  ").trim_end())
    };

    let mut out = line(&mut variables.iter().cloned());
    out.push_str(&line(&mut widths.iter().map(|w| "-".repeat(*w))));
    for row in cells {
        out.push_str(&line(&mut row.into_iter()));
    }
    out
}

/// Prefixes a query may use without declaring them, in override order: `cerbo:`,
/// then the built-in well-known prefixes, then the vault's `ontology-map.json`.
/// A later entry wins, and a `PREFIX` in the query itself wins over all.
pub fn query_prefixes(map: &PrefixMap) -> Vec<(String, String)> {
    let mut prefixes = vec![("cerbo".to_string(), NS_CERBO.to_string())];
    prefixes.extend(
        BUILTIN_PREFIXES
            .iter()
            .map(|(p, ns)| (p.to_string(), ns.to_string())),
    );
    prefixes.extend(
        map.prefixes
            .iter()
            .filter_map(|(p, e)| e.namespace.as_ref().map(|ns| (p.clone(), ns.clone()))),
    );
    // The query parser refuses a relative or malformed namespace; drop it here
    // so one bad map entry does not break every query.
    prefixes.retain(|(p, ns)| match NamedNode::new(ns.as_str()) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Warning: prefix {p}: cannot be used in queries: {e}");
            false
        }
    });
    prefixes
}

/// An evaluator with `prefixes` predeclared. They must be valid IRIs, as
/// [`query_prefixes`] guarantees.
fn evaluator(prefixes: &[(String, String)]) -> SparqlEvaluator {
    prefixes.iter().fold(SparqlEvaluator::new(), |evaluator, (prefix, namespace)| {
        evaluator
            .with_prefix(prefix, namespace)
            .expect("query_prefixes keeps only valid IRIs")
    })
}

/// Parse and evaluate `sparql` on `store`.
///
/// Without `FROM`/`FROM NAMED` the default graph is the union of all graphs,
/// so a plain pattern sees every object; a query naming its own dataset keeps
/// it. Updates are refused before anything touches the store.
pub fn run(store: &Store, sparql: &str, prefixes: &[(String, String)]) -> Result<QueryOutput, SemanticError> {
    let mut prepared = match evaluator(prefixes).parse_query(sparql) {
        Ok(prepared) => prepared,
        // Only parsed, never executed: an update is named as such instead of
        // being reported as a syntax error.
        Err(e) => {
            return Err(if evaluator(prefixes).parse_update(sparql).is_ok() {
                SemanticError::ReadOnlyQuery
            } else {
                SemanticError::QuerySyntax(e.to_string())
            });
        }
    };

    if prepared.dataset().is_default_dataset() {
        prepared.dataset_mut().set_default_graph_as_union();
    }

    let evaluation = |e: oxigraph::sparql::QueryEvaluationError| SemanticError::QueryEvaluation(e.to_string());
    match prepared.on_store(store).execute().map_err(evaluation)? {
        QueryResults::Solutions(solutions) => {
            let variables: Vec<_> = solutions.variables().to_vec();
            let mut rows = Vec::new();
            for solution in solutions {
                let solution = solution.map_err(evaluation)?;
                rows.push(variables.iter().map(|v| solution.get(v).cloned()).collect());
            }
            Ok(QueryOutput::Solutions {
                variables: variables.iter().map(|v| v.as_str().to_string()).collect(),
                rows,
            })
        }
        QueryResults::Boolean(b) => Ok(QueryOutput::Boolean(b)),
        QueryResults::Graph(triples) => Ok(QueryOutput::Graph(
            triples.collect::<Result<_, _>>().map_err(evaluation)?,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::Literal;

    fn iri(s: &str) -> Term {
        NamedNode::new_unchecked(s).into()
    }

    #[test]
    fn select_prints_an_aligned_table_with_empty_unbound_cells() {
        let output = QueryOutput::Solutions {
            variables: vec!["page".into(), "title".into()],
            rows: vec![
                vec![Some(iri("cerbo://objects/137")), Some(Literal::new_simple_literal("Berlin").into())],
                vec![Some(iri("cerbo://objects/1")), None],
            ],
        };
        assert_eq!(
            output.to_text(),
            "page                 title\n\
             -------------------  ------\n\
             cerbo://objects/137  Berlin\n\
             cerbo://objects/1\n"
        );
    }

    #[test]
    fn ask_prints_true_or_false() {
        assert_eq!(QueryOutput::Boolean(true).to_text(), "true\n");
        assert_eq!(QueryOutput::Boolean(false).to_text(), "false\n");
    }

    #[test]
    fn json_is_sparql_results_json_or_json_ld() {
        let select = QueryOutput::Solutions {
            variables: vec!["n".into()],
            rows: vec![vec![Some(Literal::from(137).into())], vec![None]],
        };
        let json: serde_json::Value = serde_json::from_str(&select.to_json()).unwrap();
        assert_eq!(json["head"]["vars"], serde_json::json!(["n"]));
        assert_eq!(json["results"]["bindings"][0]["n"]["value"], "137");
        assert_eq!(json["results"]["bindings"][1], serde_json::json!({}));

        let ask: serde_json::Value = serde_json::from_str(&QueryOutput::Boolean(true).to_json()).unwrap();
        assert_eq!(ask["boolean"], true);

        let triple = Triple::new(
            NamedNode::new_unchecked("cerbo://objects/137"),
            NamedNode::new_unchecked("https://schema.org/name"),
            Literal::new_simple_literal("Berlin"),
        );
        let graph: serde_json::Value = serde_json::from_str(&QueryOutput::Graph(vec![triple]).to_json()).unwrap();
        let text = graph.to_string();
        assert!(text.contains("cerbo://objects/137") && text.contains("https://schema.org/name"), "{text}");
    }
}
