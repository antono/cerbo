# Links and annotations

## Links: `cerbo://<uuid>`

Every object has a permanent UUID, and links point at it, so renaming a page
never breaks a link:

```markdown
Check out [this page](cerbo://123e4567-e89b-12d3-a456-426614174000) for details.
```

Both `cerbo://<uuid>` and `cerbo://objects/<uuid>` are accepted. A link to a
UUID that does not exist (or was moved to the trash) is a **broken link**: it is
reported, and nothing is created for it.

Backlinks are not stored in the vault. They are a query over the semantic
index in the user cache: the pages citing X are X's backlinks
([storage.md](storage.md#the-semantic-index-outside-the-vault)).

Titles are resolved by reading the vault's `meta.ttl` files; there is no title
index on disk to fall out of date. The desktop editor offers title completion
when you type a link.

## Annotations: `[Text]{prefix:Type}`

Cerbo supports HackMD-style semantic annotations:

```markdown
[Bob]{schema:Person} works at [Acme Corp]{schema:Organization}.
```

They are extracted into the object's `annotations.ttl`, as entities the page
mentions, using the standard schema.org vocabulary:

```turtle
<cerbo://objects/<uuid>> schema:mentions _:m0 .
_:m0 schema:name "Bob" ;
	a schema:Person ;
	cerbo:position "1:0" .
```

- The prefixes `schema`, `foaf`, `dcterms`, `skos`, `rdf`, `rdfs`, `owl` and
  `xsd` are built in.
- Other prefixes are registered per vault with
  `cerbo import-ontology <url> --prefix ex --namespace <iri>`; they are stored
  in the vault's `.cerbo/ontology-map.json`.
- An annotation with an unknown prefix (or no prefix, as in `[X]{Product}`) is
  kept as `cerbo:unresolvedType "prefix:Type"` and reported as a warning. Once
  the prefix is registered, `cerbo index` turns it into a real type.
- `cerbo:position` is `"line:column"`: the 1-based line and 0-based column of
  the opening `[`.

In the desktop app a resolved annotation renders highlighted with its type in
a tooltip; an unresolved one is underlined.

## Querying them

Annotations and links are part of the vault's knowledge graph and can be
queried with `cerbo sparql` or the desktop SPARQL console:

```bash
cerbo sparql 'SELECT ?page ?name WHERE { ?page schema:mentions ?m . ?m a schema:Place ; schema:name ?name }'
cerbo search "museums" --annotated schema:Place     # semantic search restricted by annotation
```

The exact file format is in [storage.md](storage.md#annotationsttl-turtle-rdf).
