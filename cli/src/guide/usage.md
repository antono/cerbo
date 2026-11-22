# Using a Cerbo vault: finding and reading

A Cerbo vault is a local markdown wiki. Every page has a UUID and a title. Pages link to each other as `cerbo://<uuid>`. A passage of text can carry a type: `[Berlin]{schema:Place}` says "this text is a Place". The part before the colon is a prefix (`schema`, `foaf`, or one the vault defines), the part after it is the type. The tools work on one vault only.

Other guides: {{tool:guide}} with `topic` set to `management` (changing the vault) or `ontology` (types, vocabularies and the RDF graph).

## Find things

Pick the search that fits the question. Do not read every page.

- You know the words or the title: {{tool:search_text}} (case-insensitive substring of title and text).
- You know the meaning but not the words, or the notes may be in another language: {{tool:search_semantic}}. Write the query as a short sentence about the topic, not one keyword. Scores only compare results of one query: a clear gap between the first hits and the rest means good matches; if all scores are close, or the hits look wrong, make the query more specific, or fall back to {{tool:search_text}}. If the tool says embeddings are not enabled for this vault, it cannot be used: use {{tool:search_text}} and {{tool:sparql_query}}, and tell the user that `cerbo index --embeddings` turns it on.
- You need a structured answer ("all people mentioned", "pages about places"): {{tool:sparql_query}}. Page titles are `schema:name`. An annotation is stored as `?page schema:mentions [ a <Type> ; schema:name "Text" ]`. Example: `SELECT ?page ?name WHERE { ?page schema:mentions ?x . ?x a schema:Place ; schema:name ?name }`. The query is read-only. Pages come back as `cerbo://objects/<uuid>`: the UUID is the part after `objects/`, and it is what the page tools take. The `ontology` guide has more recipes.
- You have one page and want its neighbourhood: {{tool:backlinks}} (who links here), {{tool:outgoing_links}} (what it links to).
- You want to see what exists: {{tool:list_pages}} gives `{uuid, title, virtualPath}` for every page; the virtual path is the folder. {{tool:list_attachments}} lists files attached to a page or to the whole vault.

A good order: meaning search to find candidates, {{tool:read_page}} on the best two or three, then follow links. Keep the UUIDs from results; pages are identified by UUID.

## Read a page

{{tool:read_page}} returns the title, the markdown, metadata and a `version` that identifies the exact text you read. Quote UUIDs as they are; never invent one. An unknown UUID is an error naming it.

## Limits

- You only see the tools the user allowed. If a tool you want is not there, say what you would do and let the user decide; there is no way around it.
- Say where an answer came from (page titles and UUIDs), and say when a search found nothing instead of guessing.
