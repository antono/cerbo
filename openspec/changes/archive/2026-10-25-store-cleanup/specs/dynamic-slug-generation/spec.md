# Dynamic Slug Generation

## REMOVED Requirements

### Requirement: Slug auto-generation from page name
**Reason**: This capability specifies a desktop create-page dialog whose slug field updates
live as the user types. No such dialog behaviour exists. Slug generation is specified where
it is actually implemented: `page-crud` derives `cerbo:slug` from the title at creation, and
`page-metadata-index` backfills it.

**Migration**: None. The slug algorithm and its fallback are unchanged and are stated in
`page-crud`.

### Requirement: Manual slug override disables auto-update
**Reason**: Same non-existent dialog.

**Migration**: None. A caller-supplied slug is honoured; see `page-crud`.

### Requirement: Slug validation
**Reason**: Duplicates the slug rules already stated normatively in `uuid-object-storage`,
which is where the on-disk contract belongs.

**Migration**: None. The rules are unchanged.

### Requirement: Dialog state per form instance
**Reason**: Same non-existent dialog.

**Migration**: None.

### Requirement: Real-time slug generation on keystroke
**Reason**: Same non-existent dialog.

**Migration**: None.
