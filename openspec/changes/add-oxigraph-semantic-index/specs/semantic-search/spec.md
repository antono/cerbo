# Spec Delta

## Purpose

Finds notes by meaning rather than exact words, using text embeddings computed locally, and exposes the resulting similarity as implicit links in the vault's graph index.

## ADDED Requirements

### Requirement: Embeddings are computed locally

The system SHALL compute page embeddings on the user's machine. Page text SHALL NOT be sent over the network. The only network access SHALL be the one-time download of the embedding model.

#### Scenario: Offline after model download
- **WHEN** the model has been downloaded and the machine is offline
- **THEN** `cerbo index --embeddings`, `cerbo search` and `cerbo similar` SHALL work

### Requirement: Model is downloaded on first use

The first command that needs the embedding model SHALL download it into the user's cache directory, reporting progress on stderr. Later commands SHALL reuse the cached model.

#### Scenario: First use
- **WHEN** the user runs `cerbo index --embeddings` and the model is not cached
- **THEN** the model SHALL be downloaded with progress on stderr
- **THEN** indexing SHALL continue after the download

#### Scenario: Download impossible
- **WHEN** the model is not cached and cannot be downloaded
- **THEN** the command SHALL exit non-zero with a message saying the embedding model is unavailable

### Requirement: Model files are pinned and verified

Each Cerbo release SHALL name one exact revision of the embedding model and the SHA-256 checksum of every model file. A downloaded or cached file whose checksum differs SHALL NOT be used: the system SHALL delete it and fail with an error naming the file.

#### Scenario: Tampered download
- **WHEN** a downloaded model file does not match its pinned checksum
- **THEN** the file SHALL be deleted
- **THEN** the command SHALL exit non-zero and name the file and the checksum mismatch

#### Scenario: Same model on every machine
- **WHEN** two machines run the same Cerbo release and download the model
- **THEN** both SHALL use byte-identical model files

### Requirement: Model can be provided without a download

When the environment variable `CERBO_MODEL_DIR` is set, the system SHALL load the model files from that directory, verify them against the pinned checksums, and SHALL NOT access the network for the model.

#### Scenario: Pre-provisioned model
- **WHEN** `CERBO_MODEL_DIR` points to a directory with the pinned model files and the machine is offline
- **THEN** `cerbo index --embeddings` SHALL work without any download

#### Scenario: Wrong files in the directory
- **WHEN** `CERBO_MODEL_DIR` points to files that do not match the pinned checksums
- **THEN** the command SHALL exit non-zero, name the mismatching file, and SHALL NOT fall back to a download

### Requirement: Embeddings are opt-in per vault

A vault SHALL have no embeddings until `cerbo index --embeddings` runs on it. Until then, `cerbo search` and `cerbo similar` SHALL exit non-zero and tell the user to run `cerbo index --embeddings`.

#### Scenario: Search before embeddings exist
- **WHEN** the user runs `cerbo search "trip"` in a vault without embeddings
- **THEN** the command SHALL exit non-zero and suggest `cerbo index --embeddings`

### Requirement: Saving a page does not compute embeddings

Saving a page SHALL NOT compute its embedding. Instead, `cerbo search` and `cerbo similar` SHALL first refresh the embeddings of pages whose content changed since their last embedding, so results always reflect saved content.

#### Scenario: Fresh note is found
- **WHEN** the user saves a new page about Berlin and immediately runs `cerbo search "Berlin"`
- **THEN** the new page SHALL be considered in the results

### Requirement: Search pages by meaning

`cerbo search <text>` SHALL return the pages whose content is closest in meaning to `<text>`, best first. Each result SHALL show the page UUID, title and a similarity score. `--limit <n>` SHALL bound the number of results, and SHALL default to 10.

#### Scenario: Synonym match
- **WHEN** a page titled "Trip to Berlin" exists and the user runs `cerbo search "vacation in Germany"`
- **THEN** that page SHALL appear in the results

#### Scenario: Result limit
- **WHEN** the user runs `cerbo search "notes" --limit 3`
- **THEN** at most 3 results SHALL be returned

### Requirement: Search works across Russian and English

Search SHALL match meaning across Russian and English, so a query in one language can find a page written in the other.

#### Scenario: Russian query, English page
- **WHEN** a page titled "Trip to Berlin" exists and the user runs `cerbo search "поездка в Берлин"`
- **THEN** that page SHALL appear in the results

### Requirement: Find similar pages

`cerbo similar <uuid>` SHALL return the pages closest in meaning to the given page, best first, excluding the page itself. It SHALL accept `--limit <n>`, defaulting to 10. An unknown UUID SHALL produce a non-zero exit.

#### Scenario: Similar pages exclude the source
- **WHEN** the user runs `cerbo similar <uuid>`
- **THEN** the page `<uuid>` SHALL NOT appear in the results

#### Scenario: Unknown page
- **WHEN** the user runs `cerbo similar 00000000-0000-0000-0000-000000000000`
- **THEN** the command SHALL exit non-zero with "Object not found"

### Requirement: Results can be filtered through the graph

`cerbo search` and `cerbo similar` SHALL accept `--type <prefix:Class>` to keep only objects with that `rdf:type` (for example `schema:WebPage`), and `--annotated <prefix:Type>` to keep only pages carrying an annotation of that type. Filters SHALL apply before `--limit`.

#### Scenario: Filter by annotation
- **WHEN** the user runs `cerbo search "city" --annotated schema:Place`
- **THEN** every result SHALL be a page containing a `schema:Place` annotation

#### Scenario: Filters do not shrink the result count
- **WHEN** 137 pages match the query and 20 of them carry `schema:Place`
- **AND** the user runs `cerbo search "city" --annotated schema:Place --limit 10`
- **THEN** 10 results SHALL be returned, all carrying `schema:Place`

### Requirement: JSON output for search commands

With `--json`, `cerbo search` and `cerbo similar` SHALL print only a JSON array of objects with the fields `uuid`, `title` and `score`.

#### Scenario: Search as JSON
- **WHEN** the user runs `cerbo search "trip" --json`
- **THEN** stdout SHALL be a valid JSON array whose items have `uuid`, `title` and `score`

### Requirement: Implicit similarity links in the index

After embeddings are refreshed, the index SHALL link each page to its closest pages above a similarity threshold with `cerbo:similarTo`, together with the similarity score, so SPARQL can query them. These links SHALL exist only in the index and SHALL NOT be written to vault files.

#### Scenario: Similar pages queryable by SPARQL
- **WHEN** two pages about Berlin have been embedded
- **THEN** `cerbo sparql 'SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }'` SHALL return that pair

#### Scenario: Vault files untouched
- **WHEN** similarity links are computed
- **THEN** no file in the vault SHALL contain `cerbo:similarTo`
