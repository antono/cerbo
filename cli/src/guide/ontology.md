# Ontologies, vocabularies and the RDF graph

Every page and every annotation in a vault is also stored as RDF, so you can ask structured questions with {{tool:sparql_query}} and keep a consistent vocabulary with the ontology tools.

## Known ontologies

These prefixes work in annotations and in SPARQL without any setup. {{tool:ontology_list}} shows them with the prefixes the vault added.

| Prefix | Namespace | Use it for |
| --- | --- | --- |
| `schema` | `https://schema.org/` | The default. Person, Place, Organization, Event, Book, Recipe, SoftwareApplication, CreativeWork and hundreds more. |
| `foaf` | `http://xmlns.com/foaf/0.1/` | People and groups (`foaf:Person`) when the vault already uses FOAF. |
| `dcterms` | `http://purl.org/dc/terms/` | Document metadata: creator, created, subject. |
| `skos` | `http://www.w3.org/2004/02/skos/core#` | Concepts and their relations (broader, narrower, related). |
| `rdf`, `rdfs`, `owl`, `xsd` | W3C namespaces | The language itself: `rdf:type`, `rdfs:Class`, `rdfs:label`, `rdfs:subClassOf`, datatypes such as `xsd:date`. |

Any name from these namespaces can be used in an annotation, whether or not {{tool:ontology_types}} lists it: that tool lists what a vocabulary *declares*, and the big standard ones declare nothing in the vault. Use one prefix for one kind of thing: if the vault writes people as `schema:Person`, do not introduce `foaf:Person`. The vault's own habit beats your preference; check with the queries below.

## What the RDF graph holds

- Objects are named `cerbo://objects/<uuid>` in the graph; the page tools take just the UUID.
- Each object has its own named graph `cerbo://graphs/<uuid>`. Without `FROM`, a query sees the union of all graphs.
- A page is a `schema:NoteDigitalDocument` with `schema:name` (its title). Attachments are `schema:MediaObject`, imported web pages `schema:WebPage`, vocabularies `schema:DefinedTermSet`.
- An annotation `[Alice]{foaf:Person}` is `?page schema:mentions [ a foaf:Person ; schema:name "Alice" ; cerbo:position "line:column" ]`.
- An annotation whose prefix the vault does not know is kept as `cerbo:unresolvedType "prefix:Type"` on that node, instead of `a`. These are mistakes to fix.
- Every `cerbo://` link in a page is `?page schema:citation ?target`, also when the target does not exist.
- When the vault has embeddings, the graph `cerbo://graphs/similarity` holds `?page cerbo:similarTo ?other`.
- A vocabulary's types are `rdfs:Class` declarations with `rdfs:label`, `rdfs:comment` and `rdfs:subClassOf` in the vocabulary's own graph.
- The `schema`, `foaf`, `dcterms`, `skos`, `rdf`, `rdfs`, `owl`, `xsd` and `cerbo` prefixes and every prefix of the vault need no `PREFIX` line. Queries are read-only; `SERVICE` is not supported.

## Questions worth asking

- Which types are in use, and how often (do this before choosing a type): `SELECT ?type (COUNT(?x) AS ?n) WHERE { ?page schema:mentions ?x . ?x a ?type } GROUP BY ?type ORDER BY DESC(?n)`
- Annotations that point nowhere: `SELECT ?page ?t WHERE { ?page schema:mentions ?x . ?x cerbo:unresolvedType ?t }`
- Pages that mention a kind of thing: `SELECT ?page ?name WHERE { ?page schema:mentions ?x . ?x a schema:Place ; schema:name ?name }`
- Everything a vocabulary declares: `SELECT ?class ?label WHERE { ?class a rdfs:Class . OPTIONAL { ?class rdfs:label ?label } }`
- Links to pages that do not exist: `SELECT ?page ?target WHERE { ?page schema:citation ?target . FILTER NOT EXISTS { ?target a ?any } }`
- Pages nobody links to: `SELECT ?page WHERE { ?page a schema:NoteDigitalDocument . FILTER NOT EXISTS { ?other schema:citation ?page } }`

## Looking at vocabularies

{{tool:ontology_list}} lists prefixes with the ontology object behind each and whether the vault owns it (`vaultOwned`). {{tool:ontology_types}} lists the declared types and properties with labels, comments and parents. Read these before you add anything.
<!--if write-->

## Extending the vocabulary

Create a new type only when no standard or existing type fits and you expect to use it more than once. A one-off thing is better described in the page text.

1. Find a vocabulary with `vaultOwned: true` in {{tool:ontology_list}}. If there is none, make one with {{tool:ontology_create}}: a short lowercase prefix that is not in use (not even a built-in one), and a title.
2. {{tool:ontology_add_type}} with a singular PascalCase `name` (`Recipe`), a `label`, a `comment` that says what the type means in one sentence, and a `parent` when a broader type exists (`schema:CreativeWork`). Adding a type that already exists changes nothing and returns it, so it is safe to repeat.
3. Use it at once: `[Pancakes]{ex:Recipe}`. Then run the unresolved-annotations query to confirm nothing is left dangling.

Only vocabularies the vault owns accept types. `schema`, `foaf` and the like, and vocabularies imported from a URL or another vault, refuse with a message that says so. Do not work around that by inventing a prefix that looks like a standard one.

## Importing an ontology and keeping the index healthy

- {{tool:ontology_import}} fetches an ontology file from an http or https address and stores it as an Ontology object; with `prefix` and `namespace` together it also registers the prefix, so `[Text]{prefix:Type}` resolves. It reaches the network, so it always waits for the user's approval, even in an `auto` vault: say which ontology and why, and use the address the ontology's own site publishes. An imported ontology is read-only for you: you cannot add types to it (add them to a vault-owned vocabulary, with a `parent` pointing into the imported one if that helps).
- {{tool:index_status}} tells whether the RDF graph is built, usable and current, and how many pages have embeddings. If SPARQL misses pages that exist, or the status says stale or unusable, run {{tool:index_rebuild}}; it can take a while, so do not repeat it.

## What you cannot do from here

Turning embeddings on (`cerbo index --embeddings`, which downloads a model) and registering a prefix for a namespace you do not own are user actions in the terminal or the app. When one is needed, tell the user which command and why.
<!--end-->
<!--if not write-->

## What you cannot do from here

This vault is read-only for you, so you cannot add types or vocabularies, import an ontology or rebuild the index. If a type is missing, tell the user which type, with a label and a one-sentence meaning, and which standard type it should sit under. {{tool:index_status}} still tells you whether the graph is current.
<!--end-->
