# Spec Delta

## Purpose

Lets desktop users explore the current vault's knowledge graph with read-only SPARQL queries, and jump from results straight to the pages they name.

## ADDED Requirements

### Requirement: Open and close the console

The desktop app SHALL provide a SPARQL console dialog for the current vault. It SHALL open with `Ctrl+Shift+E` (`Cmd+Shift+E` on Mac) and close with `Escape`. On open, the query field SHALL be focused and SHALL contain the last query run in this app, or an example query listing pages and titles when there is none.

#### Scenario: First open
- **WHEN** the user presses `Ctrl+Shift+E` for the first time
- **THEN** the console SHALL open with an example query listing pages and their titles

#### Scenario: Reopen keeps the last query
- **WHEN** the user runs a query, closes the console and reopens it, even after restarting the app
- **THEN** the query field SHALL contain the last query run

### Requirement: Run read-only queries

The user SHALL run the query with `Ctrl+Enter` (`Cmd+Enter` on Mac) or a Run button. Queries SHALL behave as `cerbo sparql` in the current vault, with the same predeclared prefixes. Update queries SHALL be rejected with a message. While a query runs, the Run button SHALL be disabled and the rest of the app SHALL stay usable.

#### Scenario: Update is rejected
- **WHEN** the user runs `INSERT DATA { <a> <b> <c> }`
- **THEN** the console SHALL show that only read-only queries are allowed
- **THEN** no vault file and no index content SHALL change

#### Scenario: Syntax error
- **WHEN** the user runs a query with a syntax error
- **THEN** the console SHALL show the parser message including the line and column

### Requirement: Results by query form

SELECT results SHALL be shown as a table with one column per variable and a row count. ASK results SHALL be shown as true or false. CONSTRUCT and DESCRIBE results SHALL be shown as Turtle. Results beyond 1000 rows SHALL be cut off, with a notice stating that the result was truncated.

#### Scenario: SELECT table
- **WHEN** the user runs `SELECT ?p ?t WHERE { ?p schema:name ?t }` in a vault with 137 pages
- **THEN** a table with columns `p` and `t` and 137 rows SHALL be shown, with the count 137

#### Scenario: Large result
- **WHEN** a SELECT returns more than 1000 rows
- **THEN** the first 1000 rows SHALL be shown, with a notice that the result was truncated

### Requirement: Object IRIs open pages

In a result table, a cell holding a `cerbo://objects/<uuid>` IRI of a live page SHALL show that page's title and SHALL open the page when clicked, closing the console. Other IRIs SHALL be shown in prefixed form when a predeclared prefix matches.

#### Scenario: Navigate from a result
- **WHEN** the user clicks a result cell holding a page IRI
- **THEN** the console SHALL close and that page SHALL open

#### Scenario: Prefixed display
- **WHEN** a cell holds `https://schema.org/Place`
- **THEN** it SHALL be displayed as `schema:Place`
