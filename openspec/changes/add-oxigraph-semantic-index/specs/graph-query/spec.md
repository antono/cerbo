# Spec Delta

## Purpose

Lets humans and agents ask read-only SPARQL questions about a vault's knowledge graph from the command line, with output that is readable in a terminal and parseable as JSON.

## ADDED Requirements

### Requirement: Run a SPARQL query against the vault graph

The CLI SHALL provide `cerbo sparql <query>`, which runs a SPARQL 1.1 query against the semantic index of the vault discovered from the current directory, or of the vault given with `--vault <path>`.

#### Scenario: List page titles
- **WHEN** the user runs `cerbo sparql 'SELECT ?t WHERE { ?p schema:name ?t }'`
- **THEN** stdout SHALL list the title of every indexed object

#### Scenario: Explicit vault
- **WHEN** the user runs `cerbo sparql --vault /path/to/vault '<query>'`
- **THEN** the query SHALL run against that vault's index

### Requirement: Query can come from a file or stdin

`cerbo sparql` SHALL read the query from `--file <path>`, or from stdin when the query argument is `-`, instead of from the argument.

#### Scenario: Query from file
- **WHEN** the user runs `cerbo sparql --file places.rq`
- **THEN** the query in `places.rq` SHALL run

#### Scenario: Query from stdin
- **WHEN** the user runs `echo 'ASK { ?s ?p ?o }' | cerbo sparql -`
- **THEN** the query from stdin SHALL run

### Requirement: Common prefixes are predeclared

Queries SHALL be able to use, without declaring them, the prefixes `cerbo:` (`cerbo://ontology/`), every built-in well-known prefix (including `schema:` as `https://schema.org/`, `rdf:`, `rdfs:` and `xsd:`), and every prefix in the vault's `ontology-map.json`. A vault map entry SHALL override a built-in prefix, and a `PREFIX` declared in the query SHALL override both.

#### Scenario: Undeclared cerbo prefix
- **WHEN** a query uses `schema:name` without a `PREFIX` line
- **THEN** the query SHALL run with `cerbo:` resolved to `cerbo://ontology/`

#### Scenario: schema prefix is schema.org
- **WHEN** a query uses `?a a schema:Place` without a `PREFIX` line
- **THEN** `schema:Place` SHALL resolve to `https://schema.org/Place`

#### Scenario: Vault map prefix
- **WHEN** the vault's map maps `ex` to `https://example.org/vocab#` and a query uses `ex:Person`
- **THEN** `ex:Person` SHALL resolve to `https://example.org/vocab#Person`

### Requirement: Human-readable output by default

Without `--json`, SELECT results SHALL be printed as a table with one column per variable, ASK results as `true` or `false`, and CONSTRUCT or DESCRIBE results as Turtle.

#### Scenario: ASK query
- **WHEN** the user runs `cerbo sparql 'ASK { ?s schema:name "Index" }'` and such a page exists
- **THEN** stdout SHALL be `true`

#### Scenario: CONSTRUCT query
- **WHEN** the user runs a CONSTRUCT query
- **THEN** stdout SHALL be a valid Turtle document

### Requirement: JSON output

With `--json`, SELECT and ASK results SHALL be printed as SPARQL 1.1 Query Results JSON, and nothing else SHALL be printed to stdout. CONSTRUCT and DESCRIBE results with `--json` SHALL be printed as JSON-LD.

#### Scenario: SELECT as JSON
- **WHEN** the user runs `cerbo sparql --json '<select query>'`
- **THEN** stdout SHALL be a single valid SPARQL Results JSON document

### Requirement: Queries are read-only

`cerbo sparql` SHALL reject SPARQL UPDATE operations with a non-zero exit and a message stating that vault data is changed through files, not the index.

#### Scenario: Update attempt
- **WHEN** the user runs `cerbo sparql 'INSERT DATA { <a:b> <a:c> "d" }'`
- **THEN** the command SHALL exit non-zero
- **THEN** the index SHALL be unchanged

### Requirement: Query errors are reported precisely

A query that fails to parse SHALL produce a non-zero exit and an error on stderr that includes the parser's message and position. A query that fails during evaluation SHALL also exit non-zero with the evaluation error on stderr.

#### Scenario: Syntax error
- **WHEN** the user runs `cerbo sparql 'SELEC ?s WHERE { ?s ?p ?o }'`
- **THEN** the command SHALL exit non-zero
- **THEN** stderr SHALL describe the syntax error and its position
