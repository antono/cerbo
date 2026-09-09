# Rename Cascade

## REMOVED Requirements

### Requirement: Rename page
**Reason**: The machinery this describes does not exist and cannot: it renames a slug-named
page folder and rewrites `[[OldTitle]]` occurrences across the vault using a link index.
Pages are stored under `.cerbo/objects/<uuid>/`, links are `cerbo://<uuid>`, and the link
index this requirement depends on was removed by this change. A rename now writes one
predicate in the renamed object's own `meta.ttl` and touches nothing else.

**Migration**: None. Renaming is specified by `page-crud`; no link in any other page needs
rewriting, because links carry UUIDs rather than titles.

### Requirement: Case-insensitive wikilink replacement
**Reason**: Part of the same removed cascade. No wikilink rewriting happens on rename.

**Migration**: None.
