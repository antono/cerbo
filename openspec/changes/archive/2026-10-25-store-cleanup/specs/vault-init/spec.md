# Vault Init

## REMOVED Requirements

### Requirement: Index JSON
**Reason**: `.cerbo/index.json` is never read by any code path in the project. Its
`title_to_uuid` map is not consulted for resolution, is never updated when an object is
deleted or retitled, and is therefore already permanently stale in every existing vault. Its
`uuid_to_path` map stores `objects/<uuid>/`, a value derivable from its own key. It is
rewritten in full on every object creation, making page creation O(N) in vault size. On any
parse failure the file is deleted and an empty map returned, so a single corrupt byte is
unrecoverable total loss with no rebuild path. Requiring it in the vault contract commits the
project to maintaining a file whose only effect is a class of data-loss and performance
defects.

**Migration**: No user action is required. `cerbo init` no longer creates the file. Existing
vaults keep whatever `index.json` they have; it is never read, so its presence or absence
changes no behaviour, and it may be deleted by hand at any time. Title-to-UUID resolution is
specified in the `slug-resolution` capability as a derivation from objects' `meta.ttl`, which
is what the implementation already does. Any lookup structure kept for performance is a cache
rebuildable from the objects, never an authority.
